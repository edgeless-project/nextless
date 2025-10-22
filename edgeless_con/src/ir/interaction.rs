// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub mod dialect;

#[derive(Debug, Clone)]
pub struct SourcePortMapping {
    pub dialect_type: dialect::DialectDescriptor,
    pub mapping: Box<dyn dialect::SourcePort>,
}

#[derive(Debug, Clone)]
pub struct DestiantionPortMapping {
    pub dialect_type: dialect::DialectDescriptor,
    pub mapping: Box<dyn dialect::DestinationPort>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct LogicalPortId {
    pub component: String,
    pub port: edgeless_api::function_instance::PortId,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct PhysicalPortId {
    pub instance: edgeless_api::function_instance::InstanceId,
    pub port: edgeless_api::function_instance::PortId,
}

// Derive Fails due to Clone Requirement
impl PartialEq for SourcePortMapping {
    fn eq(&self, other: &Self) -> bool {
        self.dialect_type.eq(&other.dialect_type) && self.mapping.eq(&other.mapping)
    }
}

impl Eq for SourcePortMapping {}

// Derive Fails due to Clone Requirement
impl PartialEq for DestiantionPortMapping {
    fn eq(&self, other: &Self) -> bool {
        self.dialect_type.eq(&other.dialect_type) && self.mapping.eq(&other.mapping)
    }
}

impl Eq for DestiantionPortMapping {}
