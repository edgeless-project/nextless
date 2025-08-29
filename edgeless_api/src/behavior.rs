// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct BehaviorId {
    pub id: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct EnabledPorts {
    pub enabled_inputs: std::collections::BTreeSet<crate::function_instance::PortId>,
    pub enabled_outputs: std::collections::BTreeSet<crate::function_instance::PortId>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct BehaviorImageId {
    pub behaviour_id: BehaviorId,
    pub enabled_ports: EnabledPorts,
    pub dialect_type: crate::node_registration::RuntimeType,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct BehaviorImage {
    pub behavior_image_id: BehaviorImageId,
    pub image: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct BehaviorSpec {
    pub behavior_id: BehaviorId,
    pub input_ports: std::collections::BTreeMap<crate::function_instance::PortId, crate::function_instance::Port>,
    pub output_ports: std::collections::BTreeMap<crate::function_instance::PortId, crate::function_instance::Port>,
    pub inner_structure: std::collections::BTreeMap<crate::function_instance::MappingNode, Vec<crate::function_instance::MappingNode>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct Behavior {
    pub spec: BehaviorSpec,
    pub main_image: Option<BehaviorImage>,
    pub extra_images: Vec<BehaviorImage>,
}
