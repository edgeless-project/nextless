// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Debug, serde::Deserialize)]
pub struct WorkflowSpecFunctionClass {
    pub id: String,
    pub code_type: String,
    pub version: String,
    pub code: Option<String>,
    pub build: Option<String>,
    pub outputs: std::collections::HashMap<String, PortDefinition>,
    pub inputs: std::collections::HashMap<String, PortDefinition>,
    pub inner_structure: Vec<Mapping>,
}

impl WorkflowSpecFunctionClass {
    pub fn parse(data: String) -> Self {
        serde_json::from_str(&data).unwrap()
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct PortDefinition {
    pub method: PortMethod,
    pub data_type: String,
    pub return_data_type: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub enum PortMethod {
    CAST,
    CALL,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct Mapping {
    pub source: MappingNode,
    pub dests: Vec<MappingNode>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(tag = "type", content = "port_id")]
pub enum MappingNode {
    Port(String),
    SideEffect,
}

pub trait Deserialize {
    fn deserialize(raw: &[u8]) -> Self;
}

pub trait Serialize {
    fn serialize(&self) -> Vec<u8>;
}

// pub enum EdgelessKVValue {
//     String(String),
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

impl Deserialize for std::string::String {
    fn deserialize(raw: &[u8]) -> Self {
        String::from_utf8(raw.to_vec()).unwrap()
    }
}

impl Serialize for std::string::String {
    fn serialize(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}
