// Copyright (C) 2023, 2024 Ant group. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use crate::page;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

const MERGE_PATH: &str = "/proc/uksm/merge";
const UNMERGE_PATH: &str = "/proc/uksm/unmerge";
const CMP_PATH: &str = "/proc/uksm/cmp";
const LRU_ADD_DRAIN_ALL_PATH: &str = "/proc/uksm/lru_add_drain_all";
const EPAGESNOTSAME: i32 = 541;

pub fn check_kernel() -> Result<()> {
    OpenOptions::new()
        .write(true)
        .open(MERGE_PATH)
        .map_err(|e| anyhow!("open file {} failed: {}", MERGE_PATH, e))?;

    Ok(())
}

pub fn lru_add_drain_all() -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .open(LRU_ADD_DRAIN_ALL_PATH)
        .map_err(|e| anyhow!("open file {} failed: {}", LRU_ADD_DRAIN_ALL_PATH, e))?;

    write!(file, "1")
        .map_err(|e| anyhow!("write file {} failed: {}", LRU_ADD_DRAIN_ALL_PATH, e))?;

    Ok(())
}

#[repr(C)]
struct KerneluKSMPagemapEntry {
    pme: u64,
    uksm_pme: u64,
}
const UKSM_PAGEMAP_ENTRY_SIZE: u64 = std::mem::size_of::<KerneluKSMPagemapEntry>() as u64;

const PM_PFRAME_BITS: u64 = 55;
const PM_PFRAME_MASK: u64 = (1 << PM_PFRAME_BITS) - 1;
const UKSM_CRC_BITS: u64 = 32;
const UKSM_CRC_MASK: u64 = (1 << UKSM_CRC_BITS) - 1;
const UKSM_CRC_PRESENT: u64 = 1 << 63;
const UKSM_PM_THP: u64 = 1 << 62;
const UKSM_PM_KSM: u64 = 1 << 61;

pub struct UKSMPagemapEntry {
    pub pfn: u64,
    pub crc: u32,
    pub is_thp: bool,
    pub is_ksm: bool,
}

pub fn read_uksm_pagemap(pid: u64, start: u64, end: u64) -> Result<Vec<Option<UKSMPagemapEntry>>> {
    let mut file = File::open(format!("/proc/{}/uksm_pagemap", pid))
        .map_err(|e| anyhow!("File::open failed: {}", e))?;

    let start_page_index = start / *page::PAGE_SIZE;
    let end_page_index = end / *page::PAGE_SIZE;
    let mut current_page_index = start_page_index;

    let mut buffer = vec![0; (256 * UKSM_PAGEMAP_ENTRY_SIZE) as usize];

    let mut entries = Vec::new();
    while current_page_index < end_page_index {
        let entries_to_read = std::cmp::min(256, end_page_index - current_page_index);
        let bytes_to_read = entries_to_read * UKSM_PAGEMAP_ENTRY_SIZE;
        file.seek(SeekFrom::Start(
            current_page_index * UKSM_PAGEMAP_ENTRY_SIZE,
        ))
        .map_err(|e| {
            anyhow!(
                "SeekFrom::Start {} failed: {}",
                current_page_index * UKSM_PAGEMAP_ENTRY_SIZE,
                e
            )
        })?;
        file.read_exact(&mut buffer[0..(entries_to_read * UKSM_PAGEMAP_ENTRY_SIZE) as usize])
            .map_err(|e| {
                anyhow!(
                    "file.read_exact {} {} failed: {}",
                    current_page_index * UKSM_PAGEMAP_ENTRY_SIZE,
                    entries_to_read * UKSM_PAGEMAP_ENTRY_SIZE,
                    e
                )
            })?;

        let mut index: usize = 0;
        while index < bytes_to_read as usize {
            let pme_bytes: [u8; 8] = buffer[index..(index + 8)]
                .try_into()
                .expect("Expected 8 bytes");
            let pme = u64::from_ne_bytes(pme_bytes);
            let uksm_pme_bytes: [u8; 8] = buffer[index + 8..(index + 16)]
                .try_into()
                .expect("Expected 8 bytes");
            let uksm_pme = u64::from_ne_bytes(uksm_pme_bytes);

            if uksm_pme & UKSM_CRC_PRESENT == 0 {
                entries.push(None);
            } else {
                entries.push(Some(UKSMPagemapEntry {
                    pfn: pme & PM_PFRAME_MASK,
                    crc: (uksm_pme & UKSM_CRC_MASK) as u32,
                    is_thp: uksm_pme & UKSM_PM_THP != 0,
                    is_ksm: uksm_pme & UKSM_PM_KSM != 0,
                }));
            }

            index += UKSM_PAGEMAP_ENTRY_SIZE as usize;
        }
        current_page_index += entries_to_read;
    }

    Ok(entries)
}

fn merge_pages(pa1: &PidAddr, pa2: &PidAddr) -> Result<bool> {
    let cmd = format!("{} 0x{:x} {} 0x{:x}", pa1.pid, pa1.addr, pa2.pid, pa2.addr);

    let mut cmp_file = OpenOptions::new()
        .write(true)
        .open(CMP_PATH)
        .map_err(|e| anyhow!("open file {} failed: {}", CMP_PATH, e))?;

    if let Err(e) = cmp_file.write_all(cmd.as_bytes()) {
        if let Some(errno) = e.raw_os_error() {
            if errno == EPAGESNOTSAME {
                return Ok(false);
            }
        }
        return Err(anyhow!("cmp_file.write_all {} failed: {}", cmd, e));
    }

    drop(cmp_file);

    let mut merge_file = OpenOptions::new()
        .write(true)
        .open(MERGE_PATH)
        .map_err(|e| anyhow!("open file {} failed: {}", MERGE_PATH, e))?;

    if let Err(e) = merge_file.write_all(cmd.as_bytes()) {
        if let Some(errno) = e.raw_os_error() {
            if errno == EPAGESNOTSAME {
                return Ok(false);
            }
        }
        return Err(anyhow!("merge_file.write_all {} failed: {}", cmd, e));
    }

    Ok(true)
}

fn unmerge_pages(pa: &PidAddr) -> Result<()> {
    let cmd = format!("{} 0x{:x}", pa.pid, pa.addr);

    let mut file = OpenOptions::new()
        .write(true)
        .open(UNMERGE_PATH)
        .map_err(|e| anyhow!("open file {} failed: {}", UNMERGE_PATH, e))?;

    file.write_all(cmd.as_bytes())
        .map_err(|e| anyhow!("write_all file {} {} failed: {}", UNMERGE_PATH, cmd, e))?;

    Ok(())
}

#[derive(Debug, Clone)]
struct PidAddr {
    pid: u64,
    addr: u64,
}

#[derive(Debug, Clone)]
pub struct Uksm {
    pages: HashMap<u32, Vec<Vec<PidAddr>>>,
}

impl Uksm {
    pub fn new() -> Self {
        Self {
            pages: HashMap::new(),
        }
    }

    pub fn add(&mut self, pid: u64, addr: u64, entry: &page::PageEntry) -> Result<()> {
        let new_page = PidAddr { pid, addr };

        if let Some(pagesvec) = self.pages.get_mut(&entry.crc) {
            let mut merged = false;

            'pagesvec: for pages in pagesvec.iter_mut() {
                'pages: for page in pages.iter_mut() {
                    // try to merge each pages because maybe a page in pages is updated after refresh
                    let merge_ret = merge_pages(page, &new_page)
                        .map_err(|e| anyhow!("merge_pages failed: {}", e))?;
                    if merge_ret {
                        merged = true;
                        break 'pages;
                    }
                }
                if merged {
                    pages.push(new_page.clone());
                    break 'pagesvec;
                }
            }
            if !merged {
                pagesvec.push(vec![new_page]);
            }
        } else {
            let mut pagevecs = Vec::new();
            pagevecs.push(vec![new_page]);
            self.pages.insert(entry.crc, pagevecs);
        }

        Ok(())
    }

    pub fn remove(&mut self, pid: u64, addr: u64, crc: u32) {
        let mut removed = false;
        let mut should_remove_crc = false;

        if let Some(pagesvec) = self.pages.get_mut(&crc) {
            let mut should_remove_empty_pages = false;
            for pages in pagesvec.iter_mut() {
                let origin_len = pages.len();
                pages.retain(|page| page.pid != pid || page.addr != addr);
                if origin_len != pages.len() {
                    if pages.is_empty() {
                        should_remove_empty_pages = true;
                    }
                    removed = true;
                    break;
                }
            }
            if should_remove_empty_pages {
                pagesvec.retain(|pa| !pa.is_empty());
                if pagesvec.is_empty() {
                    should_remove_crc = true;
                }
            }
        }

        if should_remove_crc {
            self.pages.remove(&crc);
        }

        if !removed {
            error!("uksm.remove cannot get {} 0x{:x} {}", pid, addr, crc);
        }
    }

    pub fn unmerge(&mut self, pid: u64, addr: u64, entry: &page::PageEntry) -> Result<()> {
        unmerge_pages(&PidAddr { pid, addr })
            .map_err(|e| anyhow!("unmerge_pages failed: {}", e))?;

        self.remove(pid, addr, entry.crc);

        Ok(())
    }
}
