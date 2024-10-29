// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod actor;
pub mod link;
pub mod managed_worflow;
pub mod proxy;
pub mod resource;
pub mod subflow;
pub mod transformations;
pub mod workflow;

pub trait LogicalComponent {
    fn logical_ports(&mut self) -> &mut LogicalPorts;
    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId>;
    fn instances(&mut self) -> Vec<&std::cell::RefCell<dyn PhysicalComponent>>;
    fn split_view(&mut self) -> (&mut LogicalPorts, Vec<&std::cell::RefCell<dyn PhysicalComponent>>);
}

pub trait PhysicalComponent {
    fn physical_ports(&mut self) -> &mut PhysicalPorts;
    fn materialized_state(&mut self) -> &mut dyn MaterializedComponent;
}

pub trait MaterializedComponent {
    fn materialized_ports(&mut self) -> &mut PhysicalPorts;
    fn runtime_statistics(&mut self) -> &mut dyn ComponentStatistics;
}

pub trait ComponentStatistics {
    fn invocation_latency(&self) -> &dyn Metric;
}

pub trait Metric {
    fn count(&self) -> u64;
    fn median(&self) -> f64;
    fn p95(&self) -> f64;
    fn p99(&self) -> f64;
    fn min(&self) -> f64;
    fn max(&self) -> f64;
}

#[derive(Default, Debug)]
pub struct LogicalPorts {
    pub logical_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalOutput>,
    pub logical_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalInput>,
}

#[derive(Default)]
pub struct PhysicalPorts {
    pub physical_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    pub physical_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
}

#[derive(Default)]
pub struct ExternalPorts {
    pub external_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
    pub external_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
}

pub struct InternalPorts {
    pub internal_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalOutput>,
    pub internal_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalInput>,
}

#[derive(Debug)]
pub enum RequiredChange {
    StartFunction {
        function_id: edgeless_api::function_instance::InstanceId,
        function_name: String,
        image: actor::ActorImage,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
        annotations: std::collections::HashMap<String, String>,
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

#[derive(Clone, Debug)]
pub enum LogicalInput {
    Direct(Vec<(String, edgeless_api::function_instance::PortId)>),
    Topic(String),
}

pub type LogicalOutput = edgeless_api::workflow_instance::PortMapping;

pub type PhysicalOutput = edgeless_api::common::Output;
pub type PhysicalInput = edgeless_api::common::Input;
