// Copyright (C) 2023, 2024 Ant group. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use crate::task;
use anyhow::{anyhow, Result};
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn pid_is_available(pid: u64) -> Result<()> {
    let maps_file = format!("/proc/{}/smaps", pid);
    File::open(maps_file.clone()).map_err(|e| anyhow!("open file {} failed: {}", maps_file, e))?;

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapRange {
    pub start: u64,
    pub end: u64,
}

struct ParseSmapsRec {
    start: u64,
    end: u64,
    anon_size: u64,
}

impl ParseSmapsRec {
    pub fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            anon_size: 0,
        }
    }

    fn invalid(&mut self) {
        self.start = 0;
        self.end = 0;
        self.anon_size = 0;
    }

    fn is_valid(&self) -> bool {
        self.anon_size != 0 && self.start != self.end
    }

    fn addr_ok(&self) -> bool {
        self.start != self.end
    }

    fn to_map_range(&self) -> MapRange {
        MapRange {
            start: self.start,
            end: self.end,
        }
    }
}

pub fn parse_task_smaps(task: &task::TaskInfo) -> Result<Vec<MapRange>> {
    let maps_file = format!("/proc/{}/smaps", task.pid);
    let file = File::open(maps_file.clone())
        .map_err(|e| anyhow!("open file {} failed: {}", maps_file, e))?;

    let reader = BufReader::new(file);
    let re = Regex::new(r"^(?P<start>[a-f0-9]+)-(?P<end>[a-f0-9]+) .*")
        .map_err(|e| anyhow!("Regex::new failed: {}", e))?;

    let mut vec: Vec<MapRange> = Vec::new();

    let mut rec = ParseSmapsRec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| anyhow!("read file {} failed: {}", maps_file, e))?;
        if let Some(captures) = re.captures(&line) {
            // Got a new vma.
            // handle the old vma rec.
            if rec.is_valid() {
                vec.push(rec.to_map_range());
            }

            rec.invalid();

            let mut start = u64::from_str_radix(&captures["start"], 16)
                .map_err(|e| anyhow!("u64::from_str_radix {} failed: {}", &captures["start"], e))?;
            let mut end = u64::from_str_radix(&captures["end"], 16)
                .map_err(|e| anyhow!("u64::from_str_radix {} failed: {}", &captures["end"], e))?;
            if start >= end {
                continue;
            }

            if let Some((tstart, tend)) = task.addr {
                if start >= tend || end <= tstart {
                    continue;
                }

                if start < tstart {
                    start = tstart;
                }

                if end > tend {
                    end = tend;
                }
            }
            rec.start = start;
            rec.end = end;
        } else if rec.addr_ok() && line.starts_with("Anonymous:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            rec.anon_size = parts[1].parse::<u64>().unwrap_or(0);
        } else if rec.addr_ok()
            && (line.starts_with("Shared_Hugetlb:") || line.starts_with("Private_Hugetlb:"))
        {
            // Ignore hugetlb vma
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            if parts[1].parse::<u64>().unwrap_or(0) > 0 {
                rec.invalid();
            }
        }
    }
    // Handle the last vma
    if rec.is_valid() {
        vec.push(rec.to_map_range());
    }

    Ok(vec)
}
