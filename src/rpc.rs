// Copyright (C) 2023, 2024 Ant group. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use crate::agent;
use crate::protocols::{empty, uksmd_ctl, uksmd_ctl_ttrpc};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use ttrpc::asynchronous::Server;
use ttrpc::error::Error;
use ttrpc::proto::Code;

#[derive(Debug)]
pub struct MyControl {
    agent: agent::Agent,
}

impl MyControl {
    pub fn new(agent: agent::Agent) -> Self {
        Self { agent }
    }
}

#[async_trait]
impl uksmd_ctl_ttrpc::Control for MyControl {
    async fn add(
        &self,
        _ctx: &::ttrpc::r#async::TtrpcContext,
        req: uksmd_ctl::AddRequest,
    ) -> ::ttrpc::Result<empty::Empty> {
        self.agent
            .send_cmd_async(agent::AgentCmd::Add(req.clone()))
            .await
            .map_err(|e| {
                let estr = format!(
                    "agent.send_cmd_async {:?} fail: {}",
                    agent::AgentCmd::Add(req),
                    e
                );
                error!("{}", estr);
                Error::RpcStatus(ttrpc::get_status(Code::INTERNAL, estr))
            })?;

        Ok(empty::Empty::new())
    }

    async fn del(
        &self,
        _ctx: &::ttrpc::r#async::TtrpcContext,
        req: uksmd_ctl::DelRequest,
    ) -> ::ttrpc::Result<empty::Empty> {
        self.agent
            .send_cmd_async(agent::AgentCmd::Del(req.clone()))
            .await
            .map_err(|e| {
                let estr = format!(
                    "agent.send_cmd_async {:?} fail: {}",
                    agent::AgentCmd::Del(req),
                    e
                );
                error!("{}", estr);
                Error::RpcStatus(ttrpc::get_status(Code::INTERNAL, estr))
            })?;

        Ok(empty::Empty::new())
    }

    async fn refresh(
        &self,
        _ctx: &::ttrpc::r#async::TtrpcContext,
        _: empty::Empty,
    ) -> ::ttrpc::Result<empty::Empty> {
        self.agent
            .send_cmd_async(agent::AgentCmd::Refresh)
            .await
            .map_err(|e| {
                let estr = format!(
                    "agent.send_cmd_async {:?} fail: {}",
                    agent::AgentCmd::Refresh,
                    e
                );
                error!("{}", estr);
                Error::RpcStatus(ttrpc::get_status(Code::INTERNAL, estr))
            })?;

        Ok(empty::Empty::new())
    }

    async fn merge(
        &self,
        _ctx: &::ttrpc::r#async::TtrpcContext,
        _: empty::Empty,
    ) -> ::ttrpc::Result<empty::Empty> {
        self.agent
            .send_cmd_async(agent::AgentCmd::Merge)
            .await
            .map_err(|e| {
                let estr = format!(
                    "agent.send_cmd_async {:?} fail: {}",
                    agent::AgentCmd::Merge,
                    e
                );
                error!("{}", estr);
                Error::RpcStatus(ttrpc::get_status(Code::INTERNAL, estr))
            })?;

        Ok(empty::Empty::new())
    }
}

#[tokio::main]
pub async fn rpc_loop(addr: String) -> Result<()> {
    let path = addr
        .strip_prefix("unix://")
        .ok_or(anyhow!("format of addr {} is not right", addr))?;
    if std::path::Path::new(path).exists() {
        return Err(anyhow!("addr {} is exist", addr));
    }

    let agent = agent::Agent::new().map_err(|e| anyhow!("agent::Agent::new fail: {}", e))?;

    let control = MyControl::new(agent);
    let c = Box::new(control) as Box<dyn uksmd_ctl_ttrpc::Control + Send + Sync>;
    let c = Arc::new(c);
    let service = uksmd_ctl_ttrpc::create_control(c);

    let mut server = Server::new().bind(&addr).unwrap().register_service(service);

    let metadata = fs::metadata(path).map_err(|e| anyhow!("fs::metadata {} fail: {}", path, e))?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions)
        .map_err(|e| anyhow!("fs::set_permissions {} fail: {}", path, e))?;

    let mut interrupt = signal(SignalKind::interrupt())
        .map_err(|e| anyhow!("signal(SignalKind::interrupt()) fail: {}", e))?;
    let mut quit = signal(SignalKind::quit())
        .map_err(|e| anyhow!("signal(SignalKind::quit()) fail: {}", e))?;
    let mut terminate = signal(SignalKind::terminate())
        .map_err(|e| anyhow!("signal(SignalKind::terminate()) fail: {}", e))?;
    server
        .start()
        .await
        .map_err(|e| anyhow!("server.start() fail: {}", e))?;

    tokio::select! {
        _ = interrupt.recv() => {
            info!("uksmd: interrupt shutdown");
        }

        _ = quit.recv() => {
            info!("uksmd: quit shutdown");
        }

        _ = terminate.recv() => {
            info!("uksmd: terminate shutdown");
        }
    };

    server
        .shutdown()
        .await
        .map_err(|e| anyhow!("server.shutdown() fail: {}", e))?;
    fs::remove_file(&path).map_err(|e| anyhow!("fs::remove_file {} fail: {}", path, e))?;

    Ok(())
}
