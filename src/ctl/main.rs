// Copyright (C) 2023, 2024 Ant group. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use structopt::StructOpt;
use ttrpc::r#async::Client;
use uksmd::protocols::{empty, uksmd_ctl, uksmd_ctl_ttrpc};

#[derive(StructOpt, Debug)]
#[structopt(name = "uksmd-ctl", about = "uKSM daemon controler")]
struct Opt {
    #[structopt(long, default_value = "unix:///var/run/uksmd.sock")]
    addr: String,

    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "add", about = "Add pid and addr")]
    Add(CommandAdd),

    #[structopt(name = "del", about = "Del task by pid")]
    Del(CommandDel),

    #[structopt(name = "refresh", about = "Refresh the page status of all tasks")]
    Refresh,

    #[structopt(name = "merge", about = "Merge the pages of all tasks")]
    Merge,
}

#[derive(StructOpt, Debug)]
struct CommandAdd {
    #[structopt(long)]
    pid: u64,
    #[structopt(long)]
    start: Option<u64>,
    #[structopt(long)]
    end: Option<u64>,
}

#[derive(StructOpt, Debug)]
struct CommandDel {
    #[structopt(long)]
    pid: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    // setup client
    let c = Client::connect(&opt.addr).unwrap();
    let client = uksmd_ctl_ttrpc::ControlClient::new(c.clone());

    match opt.command {
        Command::Add(cmdadd) => {
            if (cmdadd.start.is_none() && !cmdadd.end.is_none())
                || (!cmdadd.start.is_none() && cmdadd.end.is_none())
            {
                return Err(anyhow!(
                    "start and end should be set together or not set together"
                ));
            }
            let req = uksmd_ctl::AddRequest {
                pid: cmdadd.pid,
                OptAddr: if cmdadd.start.is_none() {
                    None
                } else {
                    Some(uksmd_ctl::add_request::OptAddr::Addr(uksmd_ctl::Addr {
                        start: cmdadd.start.unwrap_or(0),
                        end: cmdadd.end.unwrap_or(0),
                        ..Default::default()
                    }))
                },
                ..Default::default()
            };
            client
                .add(ttrpc::context::with_timeout(0), &req)
                .await
                .map_err(|e| anyhow!("client.add fail: {}", e))?;
        }

        Command::Del(cmdadd) => {
            let req: uksmd_ctl::DelRequest = uksmd_ctl::DelRequest {
                pid: cmdadd.pid,
                ..Default::default()
            };
            client
                .del(ttrpc::context::with_timeout(0), &req)
                .await
                .map_err(|e| anyhow!("client.del fail: {}", e))?;
        }

        Command::Refresh => {
            client
                .refresh(ttrpc::context::with_timeout(0), &empty::Empty::new())
                .await
                .map_err(|e| anyhow!("client.refresh fail: {}", e))?;
        }

        Command::Merge => {
            client
                .merge(ttrpc::context::with_timeout(0), &empty::Empty::new())
                .await
                .map_err(|e| anyhow!("client.merge fail: {}", e))?;
        }
    }

    Ok(())
}
