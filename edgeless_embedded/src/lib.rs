// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
#![no_std]
#![cfg_attr(feature = "nightly", feature(impl_trait_in_assoc_type))]

extern crate alloc;

const NODE_ID: uuid::Uuid = uuid::uuid!("0827240a-3050-4604-bf3e-564c41c77106");

const REGISTRATION_PEER: embassy_net::IpEndpoint = embassy_net::IpEndpoint {
    addr: embassy_net::IpAddress::v4(192, 168, 2, 61),
    port: 7001,
};

pub mod agent;
pub mod coap;
pub mod dataplane;
pub mod function_instance;
pub mod invocation;
pub mod resource;
pub mod resource_configuration;
pub mod wasm_functions;
