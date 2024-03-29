// This file is generated by ttrpc-compiler 0.6.2. Do not edit
// @generated

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unknown_lints)]
#![allow(clipto_camel_casepy)]
#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]
#![allow(clippy::all)]
use protobuf::{CodedInputStream, CodedOutputStream, Message};
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;

#[derive(Clone)]
pub struct ControlClient {
    client: ::ttrpc::r#async::Client,
}

impl ControlClient {
    pub fn new(client: ::ttrpc::r#async::Client) -> Self {
        ControlClient {
            client: client,
        }
    }

    pub async fn add(&self, ctx: ttrpc::context::Context, req: &super::uksmd_ctl::AddRequest) -> ::ttrpc::Result<super::empty::Empty> {
        let mut cres = super::empty::Empty::new();
        ::ttrpc::async_client_request!(self, ctx, req, "MemAgent.Control", "Add", cres);
    }

    pub async fn del(&self, ctx: ttrpc::context::Context, req: &super::uksmd_ctl::DelRequest) -> ::ttrpc::Result<super::empty::Empty> {
        let mut cres = super::empty::Empty::new();
        ::ttrpc::async_client_request!(self, ctx, req, "MemAgent.Control", "Del", cres);
    }

    pub async fn refresh(&self, ctx: ttrpc::context::Context, req: &super::empty::Empty) -> ::ttrpc::Result<super::empty::Empty> {
        let mut cres = super::empty::Empty::new();
        ::ttrpc::async_client_request!(self, ctx, req, "MemAgent.Control", "Refresh", cres);
    }

    pub async fn merge(&self, ctx: ttrpc::context::Context, req: &super::empty::Empty) -> ::ttrpc::Result<super::empty::Empty> {
        let mut cres = super::empty::Empty::new();
        ::ttrpc::async_client_request!(self, ctx, req, "MemAgent.Control", "Merge", cres);
    }
}

struct AddMethod {
    service: Arc<Box<dyn Control + Send + Sync>>,
}

#[async_trait]
impl ::ttrpc::r#async::MethodHandler for AddMethod {
    async fn handler(&self, ctx: ::ttrpc::r#async::TtrpcContext, req: ::ttrpc::Request) -> ::ttrpc::Result<::ttrpc::Response> {
        ::ttrpc::async_request_handler!(self, ctx, req, uksmd_ctl, AddRequest, add);
    }
}

struct DelMethod {
    service: Arc<Box<dyn Control + Send + Sync>>,
}

#[async_trait]
impl ::ttrpc::r#async::MethodHandler for DelMethod {
    async fn handler(&self, ctx: ::ttrpc::r#async::TtrpcContext, req: ::ttrpc::Request) -> ::ttrpc::Result<::ttrpc::Response> {
        ::ttrpc::async_request_handler!(self, ctx, req, uksmd_ctl, DelRequest, del);
    }
}

struct RefreshMethod {
    service: Arc<Box<dyn Control + Send + Sync>>,
}

#[async_trait]
impl ::ttrpc::r#async::MethodHandler for RefreshMethod {
    async fn handler(&self, ctx: ::ttrpc::r#async::TtrpcContext, req: ::ttrpc::Request) -> ::ttrpc::Result<::ttrpc::Response> {
        ::ttrpc::async_request_handler!(self, ctx, req, empty, Empty, refresh);
    }
}

struct MergeMethod {
    service: Arc<Box<dyn Control + Send + Sync>>,
}

#[async_trait]
impl ::ttrpc::r#async::MethodHandler for MergeMethod {
    async fn handler(&self, ctx: ::ttrpc::r#async::TtrpcContext, req: ::ttrpc::Request) -> ::ttrpc::Result<::ttrpc::Response> {
        ::ttrpc::async_request_handler!(self, ctx, req, empty, Empty, merge);
    }
}

#[async_trait]
pub trait Control: Sync {
    async fn add(&self, _ctx: &::ttrpc::r#async::TtrpcContext, _: super::uksmd_ctl::AddRequest) -> ::ttrpc::Result<super::empty::Empty> {
        Err(::ttrpc::Error::RpcStatus(::ttrpc::get_status(::ttrpc::Code::NOT_FOUND, "/MemAgent.Control/Add is not supported".to_string())))
    }
    async fn del(&self, _ctx: &::ttrpc::r#async::TtrpcContext, _: super::uksmd_ctl::DelRequest) -> ::ttrpc::Result<super::empty::Empty> {
        Err(::ttrpc::Error::RpcStatus(::ttrpc::get_status(::ttrpc::Code::NOT_FOUND, "/MemAgent.Control/Del is not supported".to_string())))
    }
    async fn refresh(&self, _ctx: &::ttrpc::r#async::TtrpcContext, _: super::empty::Empty) -> ::ttrpc::Result<super::empty::Empty> {
        Err(::ttrpc::Error::RpcStatus(::ttrpc::get_status(::ttrpc::Code::NOT_FOUND, "/MemAgent.Control/Refresh is not supported".to_string())))
    }
    async fn merge(&self, _ctx: &::ttrpc::r#async::TtrpcContext, _: super::empty::Empty) -> ::ttrpc::Result<super::empty::Empty> {
        Err(::ttrpc::Error::RpcStatus(::ttrpc::get_status(::ttrpc::Code::NOT_FOUND, "/MemAgent.Control/Merge is not supported".to_string())))
    }
}

pub fn create_control(service: Arc<Box<dyn Control + Send + Sync>>) -> HashMap<String, ::ttrpc::r#async::Service> {
    let mut ret = HashMap::new();
    let mut methods = HashMap::new();
    let streams = HashMap::new();

    methods.insert("Add".to_string(),
                    Box::new(AddMethod{service: service.clone()}) as Box<dyn ::ttrpc::r#async::MethodHandler + Send + Sync>);

    methods.insert("Del".to_string(),
                    Box::new(DelMethod{service: service.clone()}) as Box<dyn ::ttrpc::r#async::MethodHandler + Send + Sync>);

    methods.insert("Refresh".to_string(),
                    Box::new(RefreshMethod{service: service.clone()}) as Box<dyn ::ttrpc::r#async::MethodHandler + Send + Sync>);

    methods.insert("Merge".to_string(),
                    Box::new(MergeMethod{service: service.clone()}) as Box<dyn ::ttrpc::r#async::MethodHandler + Send + Sync>);

    ret.insert("MemAgent.Control".to_string(), ::ttrpc::r#async::Service{ methods, streams });
    ret
}