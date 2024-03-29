// Copyright (C) 2023, 2024 Ant group. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use crate::protocols::uksmd_ctl;
use crate::{page, proc, uksm};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TaskInfo {
    pub pid: u64,
    pub addr: Option<(u64, u64)>,
}

impl TaskInfo {
    fn new(pid: u64, addr: Option<(u64, u64)>) -> Self {
        Self { pid, addr }
    }
}

#[derive(Debug, Clone)]
enum HandleTask {
    Del(u64),
    UnMerge(u64),
    Refresh(TaskInfo),
    Merge(u64),
}

#[derive(Debug, Clone)]
struct TasksPages {
    pages_info: HashMap<u64, page::Info>,
    uksm: uksm::Uksm,
}

impl TasksPages {
    fn new() -> Self {
        Self {
            pages_info: HashMap::new(),
            uksm: uksm::Uksm::new(),
        }
    }

    fn handle_task(&mut self, ht: HandleTask) -> Result<()> {
        let mut is = page::InfoStatus::default();
        match ht.clone() {
            HandleTask::UnMerge(pid) => {
                if let Some(p) = self.pages_info.get_mut(&pid) {
                    p.unmerge(&mut self.uksm)
                        .map_err(|e| anyhow!("p.unmerge failed: {}", e))?;
                    is = p.get_status();
                }
            }
            HandleTask::Del(pid) => {
                self.pages_info.remove(&pid);
            }
            HandleTask::Refresh(task) => {
                if !self.pages_info.contains_key(&task.pid) {
                    self.pages_info.insert(task.pid, page::Info::new(task.pid));
                }

                if let Some(p) = self.pages_info.get_mut(&task.pid) {
                    p.refresh(&mut self.uksm, task)
                        .map_err(|e| anyhow!("p.refresh failed: {}", e))?;
                    is = p.get_status();
                }
            }
            HandleTask::Merge(pid) => {
                if let Some(p) = self.pages_info.get_mut(&pid) {
                    p.merge(&mut self.uksm)
                        .map_err(|e| anyhow!("p.merge failed: {}", e))?;
                    is = p.get_status();
                }
            }
        }

        trace!("handle_task {:?} result {:?}", ht, is);

        Ok(())
    }
}

#[derive(Debug, Clone)]
enum AsyncWork {
    UnMerge,
    Del,
    Refresh,
    Merge,
}

#[derive(Debug, Clone)]
pub struct Tasks {
    // map pid to Task
    map: Arc<RwLock<HashMap<u64, TaskInfo>>>,

    // tasks should refresh
    refresh_target: Arc<Mutex<Vec<TaskInfo>>>,

    // tasks should add to uksm
    merge_target: Arc<Mutex<Vec<u64>>>,

    // tasks should unmerge
    unmerge_target: Arc<Mutex<Vec<u64>>>,

    // tasks should del from tasks_pages
    del_target: Arc<Mutex<Vec<u64>>>,

    tasks_pages: Arc<Mutex<TasksPages>>,
}

impl Tasks {
    pub fn new() -> Self {
        Self {
            map: Arc::new(RwLock::new(HashMap::new())),
            refresh_target: Arc::new(Mutex::new(Vec::new())),
            merge_target: Arc::new(Mutex::new(Vec::new())),
            unmerge_target: Arc::new(Mutex::new(Vec::new())),
            del_target: Arc::new(Mutex::new(Vec::new())),
            tasks_pages: Arc::new(Mutex::new(TasksPages::new())),
        }
    }

    pub async fn add(&mut self, req: uksmd_ctl::AddRequest) -> Result<()> {
        let mut addr = None;
        if let Some(oaddr) = req.OptAddr {
            match oaddr {
                uksmd_ctl::add_request::OptAddr::Addr(raddr) => {
                    addr = Some((raddr.start, raddr.end));
                }
            }
        }

        proc::pid_is_available(req.pid)
            .map_err(|e| anyhow!("proc::pid_is_available {} failed: {}", req.pid, e))?;
        if let Some((start, end)) = addr {
            if start % *page::PAGE_SIZE != 0 || end % *page::PAGE_SIZE != 0 {
                return Err(anyhow!("start {} or end {} is not right", start, end));
            }
        }

        {
            let mut map = self.map.write().await;
            if map.contains_key(&req.pid) {
                return Err(anyhow!("pid {} exists", req.pid));
            }

            map.insert(req.pid, TaskInfo::new(req.pid, addr));
        }

        self.refresh_target
            .lock()
            .await
            .push(TaskInfo::new(req.pid, addr));

        Ok(())
    }

    pub async fn del(&mut self, req: uksmd_ctl::DelRequest) -> Result<()> {
        let mut map = self.map.write().await;

        if let Some(_) = map.remove(&req.pid) {
            self.refresh_target
                .lock()
                .await
                .retain(|task| task.pid != req.pid);
            self.merge_target.lock().await.retain(|pid| *pid != req.pid);
            self.unmerge_target
                .lock()
                .await
                .retain(|pid| *pid != req.pid);

            self.unmerge_target.lock().await.push(req.pid);
            self.del_target.lock().await.push(req.pid);
        } else {
            return Err(anyhow!("pid {} does not exist", req.pid));
        }

        Ok(())
    }

    pub async fn add_refresh_all(&mut self) {
        let mut set: HashSet<TaskInfo> = self
            .map
            .write()
            .await
            .clone()
            .into_iter()
            .map(|(_, v)| v)
            .collect();

        let mut target = self.refresh_target.lock().await;

        for t in target.clone() {
            set.insert(t);
        }

        *target = set.into_iter().collect();
    }

    pub async fn add_merge_all(&mut self) {
        let mut set: HashSet<u64> = self
            .map
            .write()
            .await
            .clone()
            .into_iter()
            .map(|(k, _)| k)
            .collect();

        let mut target = self.merge_target.lock().await;

        for t in target.clone() {
            set.insert(t);
        }

        *target = set.into_iter().collect();
    }

    fn async_work_thread(&mut self, work: AsyncWork) -> Result<()> {
        if let AsyncWork::Merge = work {
            uksm::lru_add_drain_all()?;
        }

        loop {
            let ht = {
                match work {
                    AsyncWork::UnMerge => {
                        if let Some(pid) = self.unmerge_target.blocking_lock().pop() {
                            HandleTask::UnMerge(pid)
                        } else {
                            break;
                        }
                    }
                    AsyncWork::Del => {
                        if let Some(pid) = self.del_target.blocking_lock().pop() {
                            HandleTask::Del(pid)
                        } else {
                            break;
                        }
                    }
                    AsyncWork::Refresh => {
                        if let Some(t) = self.refresh_target.blocking_lock().pop() {
                            HandleTask::Refresh(t)
                        } else {
                            break;
                        }
                    }
                    AsyncWork::Merge => {
                        if let Some(pid) = self.merge_target.blocking_lock().pop() {
                            HandleTask::Merge(pid)
                        } else {
                            break;
                        }
                    }
                }
            };

            if let Err(e) = self.tasks_pages.blocking_lock().handle_task(ht.clone()) {
                error!("handle_task {:?} failed: {}", ht, e)
            }
        }

        Ok(())
    }

    //merge: true is merge, false is refresh
    pub async fn async_work(&mut self, ret_tx: mpsc::Sender<Result<()>>) -> bool {
        let work = if self.unmerge_target.lock().await.len() > 0 {
            AsyncWork::UnMerge
        } else if self.del_target.lock().await.len() > 0 {
            AsyncWork::Del
        } else if self.refresh_target.lock().await.len() > 0 {
            AsyncWork::Refresh
        } else if self.merge_target.lock().await.len() > 0 {
            AsyncWork::Merge
        } else {
            return false;
        };

        let mut tasks = self.clone();

        thread::spawn(move || {
            info!("async_work_thread {:?} start", work);

            let ret = tasks.async_work_thread(work.clone());

            if let Err(e) = ret_tx.blocking_send(ret) {
                error!(
                    "async_work_thread {:?} ret_tx.blocking_send failed: {}",
                    work, e
                );
                return;
            }

            info!("async_work_thread {:?} stop", work);
        });

        true
    }
}
