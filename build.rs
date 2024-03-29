// Copyright (C) 2023 Ant group. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use ttrpc_codegen::{Codegen, Customize, ProtobufCustomize};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protos = vec![
        "src/protocols/protos/uksmd_ctl.proto",
        "src/protocols/protos/google/protobuf/empty.proto",
    ];

    let protobuf_customized = ProtobufCustomize::default().gen_mod_rs(false);

    Codegen::new()
        .out_dir("src/protocols/")
        .inputs(&protos)
        .include("src/protocols/protos/")
        .rust_protobuf()
        .customize(Customize {
            async_all: true,
            ..Default::default()
        })
        .rust_protobuf_customize(protobuf_customized.clone())
        .run()?;

    Ok(())
}
