// Copyright (C) 2023, 2024 Ant group. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
use anyhow::{anyhow, Result};
use log4rs::{
    append::console::ConsoleAppender,
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};
use structopt::StructOpt;

mod agent;
mod page;
mod proc;
mod protocols;
mod rpc;
mod task;
mod uksm;

#[derive(StructOpt, Debug)]
#[structopt(name = "uksmd", about = "uKSM daemon")]
struct Opt {
    #[structopt(long, default_value = "unix:///var/run/uksmd.sock")]
    addr: String,
    #[structopt(long)]
    log_file: Option<String>,
    #[structopt(long, default_value = "Trace")]
    log_level: log::LevelFilter,
}

pub const LOG_FORMAT: &str = "{d} [{l}] {f}:{L} - {m}{n}";

fn setup_logging(opt: &Opt) -> Result<()> {
    let config = if let Some(f) = &opt.log_file {
        let file_appender = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(LOG_FORMAT)))
            .build(f)
            .map_err(|e| anyhow!("FileAppender::builder() file {} fail: {}", f, e))?;

        Config::builder()
            .appender(Appender::builder().build("file", Box::new(file_appender)))
            .build(Root::builder().appender("file").build(opt.log_level))
            .map_err(|e| anyhow!("Config::builder file_appender fail: {}", e))?
    } else {
        let stderr_appender = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new(LOG_FORMAT)))
            .build();

        Config::builder()
            .appender(Appender::builder().build("stderr", Box::new(stderr_appender)))
            .build(Root::builder().appender("stderr").build(opt.log_level))
            .map_err(|e| anyhow!("Config::builder stderr_appender fail: {}", e))?
    };

    log4rs::init_config(config).map_err(|e| anyhow!("log4rs::init_config fail: {}", e))?;

    Ok(())
}

fn main() -> Result<()> {
    // Check opt
    let opt = Opt::from_args();

    setup_logging(&opt).map_err(|e| anyhow!("setup_logging fail: {}", e))?;

    uksm::check_kernel().map_err(|e| anyhow!("uksm::check_kernel fail: {}", e))?;

    info!("uKSM daemon start");

    rpc::rpc_loop(opt.addr).map_err(|e| {
        let estr = format!("rpc::grpc_loop fail: {}", e);
        error!("{}", estr);
        anyhow!("{}", estr)
    })?;

    info!("uKSM daemon stop");

    Ok(())
}
