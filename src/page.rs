// Copyright (C) 2024, 2024 Ant group. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use crate::proc::MapRange;
use crate::{proc, task, uksm};
use anyhow::{anyhow, Result};
use page_size;
use std::collections::HashMap;

lazy_static! {
    pub static ref PAGE_SIZE: u64 = page_size::get() as u64;
}

#[derive(Debug, Clone)]
pub struct PageEntry {
    pub crc: u32,
}

#[derive(Default, Debug)]
pub struct InfoStatus {
    pub new_count: u64,
    pub old_count: u64,
    pub uksm_count: u64,
}

#[derive(Debug, Clone)]
pub struct Info {
    pid: u64,
    maps: Vec<proc::MapRange>,
    new_pages: HashMap<u64, PageEntry>,
    old_pages: HashMap<u64, PageEntry>,
    uksm_pages: HashMap<u64, PageEntry>,
}

impl Info {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            maps: Vec::new(),
            new_pages: HashMap::new(),
            old_pages: HashMap::new(),
            uksm_pages: HashMap::new(),
        }
    }

    fn remove(&mut self, uksm: &mut uksm::Uksm, addr: u64) {
        if let Some(_) = self.new_pages.remove(&addr) {
            return;
        }

        if let Some(_) = self.old_pages.remove(&addr) {
            return;
        }

        if let Some(e) = self.uksm_pages.remove(&addr) {
            uksm.remove(self.pid, addr, e.crc);
        }
    }

    fn remove_maps(&mut self, uksm: &mut uksm::Uksm, maps: Vec<MapRange>) {
        for map in maps {
            for addr in (map.start..map.end).step_by(*PAGE_SIZE as usize) {
                self.remove(uksm, addr);
            }
        }
    }

    fn update(&mut self, uksm: &mut uksm::Uksm, addr: u64, entry: uksm::UKSMPagemapEntry) {
        if let Some(e) = self.new_pages.get_mut(&addr) {
            if e.crc != entry.crc {
                e.crc = entry.crc;
            } else if let Some(value) = self.new_pages.remove(&addr) {
                self.old_pages.insert(addr, value);
            }
            return;
        }

        if let Some(e) = self.old_pages.get_mut(&addr) {
            if e.crc != entry.crc {
                e.crc = entry.crc;
                if let Some(value) = self.old_pages.remove(&addr) {
                    self.new_pages.insert(addr, value);
                }
            }
            return;
        }

        if let Some(e) = self.uksm_pages.get_mut(&addr) {
            if !entry.is_ksm || e.crc != entry.crc {
                uksm.remove(self.pid, addr, e.crc);

                e.crc = entry.crc;
                if let Some(value) = self.uksm_pages.remove(&addr) {
                    self.new_pages.insert(addr, value);
                }
            }

            return;
        }

        self.new_pages.insert(addr, PageEntry { crc: entry.crc });
    }

    pub fn refresh(&mut self, uksm: &mut uksm::Uksm, task: task::TaskInfo) -> Result<()> {
        let maps = proc::parse_task_smaps(&task)
            .map_err(|e| anyhow!("proc::parse_task_smaps failed: {}", e))?;

        let should_remove_maps = find_non_overlapping_ranges(&self.maps, &maps);

        self.remove_maps(uksm, should_remove_maps);

        let mut new_maps = Vec::new();
        for r in maps {
            let entries = uksm::read_uksm_pagemap(task.pid, r.start, r.end).map_err(|e| {
                anyhow!("uksm::read_uksm_pagemap {} {:?} failed: {}", task.pid, r, e)
            })?;

            let mut addr = r.start;
            let mut current_map_is_empty = true;
            for e in entries {
                if let Some(entry) = e {
                    current_map_is_empty = false;
                    self.update(uksm, addr, entry);
                } else {
                    self.remove(uksm, addr);
                }
                addr += *PAGE_SIZE;
            }

            if !current_map_is_empty {
                new_maps.push(r);
            }
        }

        self.maps = new_maps;

        Ok(())
    }

    pub fn merge(&mut self, uksm: &mut uksm::Uksm) -> Result<()> {
        let addrs: Vec<_> = self.old_pages.keys().cloned().collect();

        for addr in addrs {
            if let Some(entry) = self.old_pages.get(&addr) {
                uksm.add(self.pid, addr, entry)?;
            }

            if let Some(entry) = self.old_pages.remove(&addr) {
                self.uksm_pages.insert(addr, entry);
            }
        }

        Ok(())
    }

    pub fn unmerge(&mut self, uksm: &mut uksm::Uksm) -> Result<()> {
        let addrs: Vec<_> = self.uksm_pages.keys().cloned().collect();

        for addr in addrs {
            if let Some(entry) = self.uksm_pages.get(&addr) {
                uksm.unmerge(self.pid, addr, entry)?;
            }

            if let Some(entry) = self.uksm_pages.remove(&addr) {
                self.old_pages.insert(addr, entry);
            }
        }

        Ok(())
    }

    pub fn get_status(&self) -> InfoStatus {
        InfoStatus {
            new_count: self.new_pages.len() as u64,
            old_count: self.old_pages.len() as u64,
            uksm_count: self.uksm_pages.len() as u64,
        }
    }
}

fn find_non_overlapping_ranges(
    a: &Vec<proc::MapRange>,
    b: &Vec<proc::MapRange>,
) -> Vec<proc::MapRange> {
    let mut c: Vec<proc::MapRange> = Vec::new();

    for range_a in a.iter() {
        let mut current_start = range_a.start;
        let mut overlaps = b
            .iter()
            .filter(|range_b| range_b.start < range_a.end && range_b.end > range_a.start)
            .collect::<Vec<_>>();

        // Sort overlapping ranges based on their start to process them in order.
        overlaps.sort_by_key(|k| k.start);

        for range_b in overlaps {
            // If the current start is less than the start of the overlapping range, then we have a non-overlapping part.
            if current_start < range_b.start {
                c.push(proc::MapRange {
                    start: current_start,
                    end: range_b.start,
                });
            }
            // Update the current start to the end of the overlapping range, if it's greater.
            if current_start < range_b.end {
                current_start = range_b.end;
            }
        }

        // If there's any remaining non-overlapping part, add it to the result.
        if current_start < range_a.end {
            c.push(proc::MapRange {
                start: current_start,
                end: range_a.end,
            });
        }
    }

    c
}
