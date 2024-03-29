// Copyright (C) 2023, 2024 Ant group. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use crate::protocols::uksmd_ctl;
use crate::task;
use anyhow::{anyhow, Result};
use tokio::runtime::{Builder, Runtime};
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum AgentCmd {
    Add(uksmd_ctl::AddRequest),
    Del(uksmd_ctl::DelRequest),
    Refresh,
    Merge,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum AgentReturn {
    Ok,
    Err(anyhow::Error),
}

async fn agent_loop(
    mut cmd_rx: mpsc::Receiver<(AgentCmd, oneshot::Sender<AgentReturn>)>,
) -> Result<()> {
    let mut tasks = task::Tasks::new();

    let (work_ret_tx, mut work_ret_rx) = mpsc::channel(2);
    let mut work_is_running = false;

    loop {
        select! {
            Some((cmd, ret_tx)) = cmd_rx.recv() => {
                let mut ret_msg = AgentReturn::Ok;
                match cmd {
                    AgentCmd::Add(req) => {
                        if let Err(e) = tasks.add(req).await {
                            ret_msg = AgentReturn::Err(e);
                        }
                    }
                    AgentCmd::Del(req) => {
                        if let Err(e) = tasks.del(req).await {
                            ret_msg = AgentReturn::Err(e);
                        }
                    }
                    AgentCmd::Refresh => {
                        tasks.add_refresh_all().await;
                    }
                    AgentCmd::Merge => {
                        tasks.add_refresh_all().await;
                        tasks.add_merge_all().await;
                    }
                }
                ret_tx.send(ret_msg).map_err(|e| anyhow!("ret_tx.send failed: {:?}", e))?;
            }
            Some(work_ret) = work_ret_rx.recv() => {
                work_is_running = false;
                if let Err(e) = work_ret {
                    error!("work task error {}", e);
                }
            }
        }

        if !work_is_running {
            work_is_running = tasks.async_work(work_ret_tx.clone()).await;
        }
    }
}

#[derive(Debug)]
pub struct Agent {
    _rt: Runtime,
    cmd_tx: mpsc::Sender<(AgentCmd, oneshot::Sender<AgentReturn>)>,
}

impl Agent {
    pub fn new() -> Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel(10);

        let rt = Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|e| anyhow!("Builder::new_multi_thread failed: {}", e))?;

        rt.spawn(async move {
            info!("uKSM agent start");
            match agent_loop(cmd_rx).await {
                Err(e) => error!("uKSM agent error {}", e),
                Ok(()) => info!("uKSM agent stop"),
            }
        });

        Ok(Self { cmd_tx, _rt: rt })
    }

    pub async fn send_cmd_async(&self, cmd: AgentCmd) -> Result<AgentReturn> {
        let (ret_tx, ret_rx) = oneshot::channel();

        self.cmd_tx
            .send((cmd, ret_tx))
            .await
            .map_err(|e| anyhow!("cmd_tx.send cmd failed: {}", e))?;

        let ret = ret_rx
            .await
            .map_err(|e| anyhow!("ret_rx.recv failed: {}", e))?;

        Ok(ret)
    }
}
