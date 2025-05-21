// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#![no_std]
extern crate alloc;

#[derive(Debug, serde::Deserialize)]
pub struct WorkflowSpecFunctionClass {
    pub id: alloc::string::String,
    pub code_type: alloc::string::String,
    pub version: alloc::string::String,
    pub code: Option<alloc::string::String>,
    pub build: Option<alloc::string::String>,
    pub outputs: alloc::collections::BTreeMap<alloc::string::String, PortDefinition>,
    pub inputs: alloc::collections::BTreeMap<alloc::string::String, PortDefinition>,
    pub inner_structure: alloc::vec::Vec<Mapping>,
}

impl WorkflowSpecFunctionClass {
    pub fn parse(data: alloc::string::String) -> Self {
        serde_json::from_str(&data).unwrap()
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct PortDefinition {
    pub method: PortMethod,
    pub data_type: alloc::string::String,
    pub return_data_type: Option<alloc::string::String>,
}

#[derive(Debug, serde::Deserialize)]
pub enum PortMethod {
    CAST,
    CALL,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct Mapping {
    pub source: MappingNode,
    pub dests: alloc::vec::Vec<MappingNode>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(tag = "type", content = "port_id")]
pub enum MappingNode {
    #[serde(rename = "SIDE_EFFECT")]
    SideEffect,
    #[serde(rename = "PORT")]
    Port(alloc::string::String),
}

pub trait Deserialize<'a> {
    fn deserialize(raw: &'a [u8]) -> Self;
}

pub trait Serialize<'a> {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]>;
}

// pub enum EdgelessKVValue {
//     alloc::string::String(alloc::string::String),
//     Float(f64),
//     Signed(i64),
//     Unsigned(u64),
//     Boolean(bool),
//     Struct(Box<EdgelessKVValue>)
// }

// pub trait EdgelessKVType {
//     fn get(key: &str) -> Option<EdgelessKVValue>;
//     fn set(key: &str, val: EdgelessKVValue);
// }

impl Deserialize<'_> for alloc::string::String {
    fn deserialize(raw: &[u8]) -> Self {
        alloc::string::String::from_utf8(raw.to_vec()).unwrap()
    }
}

impl<'a> Serialize<'a> for alloc::string::String {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        self.as_bytes()
    }
}

impl<'a> Serialize<'a> for () {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        &[]
    }
}

impl Deserialize<'_> for () {
    fn deserialize(_raw: &[u8]) -> Self {}
}

impl<'a> Serialize<'a> for &'a str {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        self.as_bytes()
    }
}

impl<'a> Deserialize<'a> for &'a str {
    fn deserialize(raw: &'a [u8]) -> Self {
        core::str::from_utf8(raw).unwrap()
    }
}
