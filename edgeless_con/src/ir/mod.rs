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
    fn materialized_state(&self) -> Option<&std::cell::RefCell<dyn MaterializedComponent>>;
}

pub trait MaterializedComponent {
    fn materialized_ports(&mut self) -> &mut MaterializedPorts;
    fn runtime_statistics(&self) -> Option<&dyn ComponentRuntimeStatistics>;
}

pub trait ComponentRuntimeStatistics: Sync + Send {
    fn invocation_rate_abs(&self, period: std::time::Duration) -> Option<f64>;
    fn invocations_rate_abs_by_port(&self, period: std::time::Duration) -> Vec<(edgeless_api::function_instance::PortId, f64)>;
    fn duration_mean_secs(&self, period: std::time::Duration) -> Option<f64>;
    fn duration_soft_limit_rate_rel(&self, period: std::time::Duration) -> Option<f64>;
    fn error_rate_rel(&self, period: std::time::Duration) -> Option<f64>;

    // TODO(raphael) Add Memory Tracking
    // fn memory_top_mean_bytes(&self, period: std::time::Duration) -> Option<f64>;
    // fn memory_soft_limit_rate_rel(&self, period: std::time::Duration) -> Option<f64>;
}

pub trait PortStatistics: Sync + Send {
    fn message_rate_abs(&self, period: std::time::Duration) -> Option<f64>;
    fn message_rate_abs_by_peer(&self, period: std::time::Duration) -> Vec<(edgeless_api::function_instance::InstanceId, f64)>;
    fn message_size_mean_bytes(&self, period: std::time::Duration) -> Option<f64>;
    fn message_size_mean_byte_by_peer(&self, period: std::time::Duration) -> Vec<(edgeless_api::function_instance::InstanceId, f64)>;

    // TODO: Probably no great way to meaure this (one-way latencies)
    // fn latency_mean_secs(&self, period: Option<std::time::Duration>) -> f64;
    // fn latency_by_peer_mean_secs(&self, period: Option<std::time::Duration>) -> Vec<(edgeless_api::function_instance::InstanceId, f64)>;
}

// Read-only view of a node's state
pub trait Node {
    // fn capabilities(&self) -> &edgeless_api::node_registration::NodeCapabilities;
    // Available Hardware Resources
    // Available Software (Runtimes, Resources)
    // Available Links
    // Usage
}

// Read-only view of a peer-cluster's state
pub trait Cluster {}

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

pub struct MaterializedPorts {
    pub materialized_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, MaterializedOutput>,
    pub materialized_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, MaterializedInput>,
}

impl MaterializedPorts {
    fn is_current_mapping(&self, other: &PhysicalPorts) -> bool {
        if self.materialized_outputs.len() != other.physical_output_mapping.len()
            || self.materialized_inputs.len() != other.physical_input_mapping.len()
        {
            return false;
        }

        for (k, v) in &self.materialized_outputs {
            match other.physical_output_mapping.get(k) {
                Some(ov) => {
                    if v.mapping != *ov {
                        return false;
                    }
                }
                _ => {
                    return false;
                }
            }
        }

        for (k, v) in &self.materialized_inputs {
            match other.physical_input_mapping.get(k) {
                Some(ov) => {
                    if v.mapping != *ov {
                        return false;
                    }
                }
                _ => {
                    return false;
                }
            }
        }

        return true;
    }
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

pub struct MaterializedInput {
    pub(crate) mapping: edgeless_api::common::Input,
    port_statistics: Option<Box<dyn PortStatistics>>,
}

impl MaterializedInput {
    fn runtime_statistics(&self) -> Option<&dyn PortStatistics> {
        self.port_statistics.as_deref()
    }
}

pub struct MaterializedOutput {
    pub(crate) mapping: edgeless_api::common::Output,
    port_statistics: Option<Box<dyn PortStatistics>>,
}

impl MaterializedOutput {
    fn runtime_statistics(&self) -> Option<&dyn PortStatistics> {
        self.port_statistics.as_deref()
    }
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
