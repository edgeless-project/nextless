// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod actor;
pub mod behavior;
pub mod component;
pub mod interaction;
pub mod link;
pub mod logical_model;
pub mod managed_worflow;
pub mod physical_model;
pub mod pipeline;
pub mod proxy;
pub mod resource;
pub mod subflow;
pub mod support;
pub mod system_model;
pub mod transformations;
pub mod workflow;

#[cfg(test)]
mod test;

pub use logical_model::*;
pub use physical_model::*;
pub use system_model::*;

pub trait TelemetryProvider: TelemetryProviderClone + Sync + Send {
    fn component_statistics_for(&self, component_id: &edgeless_api::function_instance::InstanceId) -> Box<dyn ComponentRuntimeStatistics>;
    fn input_port_statistics_for(
        &self,
        component_id: &edgeless_api::function_instance::InstanceId,
        port_id: &edgeless_api::function_instance::PortId,
    ) -> Box<dyn PortStatistics>;
    fn output_port_statistics_for(
        &self,
        component_id: &edgeless_api::function_instance::InstanceId,
        port_id: &edgeless_api::function_instance::PortId,
    ) -> Box<dyn PortStatistics>;
    fn wasm_runtime_statistics_for(&self, node_id: &edgeless_api::function_instance::NodeId) -> Box<dyn WasmRuntimeInfo>;
}

// https://stackoverflow.com/a/30353928
pub trait TelemetryProviderClone {
    fn clone_box(&self) -> Box<dyn TelemetryProvider>;
}
impl<T> TelemetryProviderClone for T
where
    T: 'static + TelemetryProvider + Clone,
{
    fn clone_box(&self) -> Box<dyn TelemetryProvider> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn TelemetryProvider> {
    fn clone(&self) -> Box<dyn TelemetryProvider> {
        self.clone_box()
    }
}

#[derive(Default, Clone, Debug)]
#[allow(unused)]
pub struct ExternalPorts {
    pub external_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
    pub external_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
}

#[derive(Default, Clone, Debug)]
#[allow(unused)]
pub struct InternalPorts {
    pub internal_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalOutput>,
    pub internal_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalInput>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum RequiredChange {
    StartFunction {
        function_id: edgeless_api::function_instance::InstanceId,
        function_name: String,
        image: behavior::BehaviorImage,
        behavior_spec: behavior::BehaviorSpec,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
        annotations: std::collections::HashMap<String, String>,
    },
    StopFunction {
        function_id: edgeless_api::function_instance::InstanceId,
    },
    StopResource {
        resource_id: edgeless_api::function_instance::InstanceId,
    },
    StartResource {
        resource_id: edgeless_api::function_instance::InstanceId,
        resource_name: String,
        class_type: String,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
        configuration: std::collections::HashMap<String, String>,
    },
    PatchFunction {
        function_id: edgeless_api::function_instance::InstanceId,
        function_name: String,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    },
    PatchResource {
        resource_id: edgeless_api::function_instance::InstanceId,
        resource_name: String,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    },
    InstantiateLinkControlPlane {
        link_id: edgeless_api::link::LinkInstanceId,
        class: edgeless_api::link::LinkType,
    },
    CreateLinkOnNode {
        link_id: edgeless_api::link::LinkInstanceId,
        node_id: edgeless_api::function_instance::NodeId,
        provider_id: edgeless_api::link::LinkProviderId,
        config: Vec<u8>,
    },
    #[allow(unused)]
    RemoveLinkFromNode {
        link_id: edgeless_api::link::LinkInstanceId,
        node_id: edgeless_api::function_instance::NodeId,
    },
    CreateSubflow {
        subflow_id: edgeless_api::function_instance::InstanceId,
        spawn_req: edgeless_api::workflow_instance::SpawnWorkflowRequest,
    },
    PatchSubflow {
        subflow_id: edgeless_api::function_instance::InstanceId,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    },
    PatchProxy {
        proxy_id: edgeless_api::function_instance::InstanceId,
        internal_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        internal_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
        external_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        external_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    },
    CrateProxy {
        proxy_id: edgeless_api::function_instance::InstanceId,
        internal_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        internal_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
        external_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        external_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    },
}
