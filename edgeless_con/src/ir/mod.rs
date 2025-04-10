// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
#![allow(unused)]

pub mod actor;
pub mod link;
pub mod managed_worflow;
pub mod pipeline;
pub mod proxy;
pub mod resource;
pub mod subflow;
pub mod transformations;
pub mod workflow;

pub trait LogicalComponent {
    fn logical_ports(&mut self) -> &mut LogicalPorts;
    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId>;
    fn instances(&mut self) -> Vec<&std::cell::RefCell<dyn MaybePhyiscalInstance>>;
    fn split_view(&mut self) -> (&mut LogicalPorts, Vec<&std::cell::RefCell<dyn MaybePhyiscalInstance>>);
}

pub enum PhysicalComponentState<C: PhysicalComponent> {
    Planned,
    Existing(C),
    StopPlanned(C),
    Stopped(C),
}

pub trait MaybePhyiscalInstance {
    fn try_unpack(&mut self) -> Option<&mut dyn PhysicalComponent>;
    fn id(&self) -> Option<edgeless_api::function_instance::InstanceId>;
}

impl<C: PhysicalComponent> MaybePhyiscalInstance for PhysicalComponentState<C>
where
    C: PhysicalComponent,
{
    fn try_unpack(&mut self) -> Option<&mut dyn PhysicalComponent> {
        match self {
            PhysicalComponentState::Planned => None,
            PhysicalComponentState::Existing(inner) => Some(inner),
            PhysicalComponentState::StopPlanned(inner) => Some(inner),
            PhysicalComponentState::Stopped(inner) => Some(inner),
        }
    }

    fn id(&self) -> Option<edgeless_api::function_instance::InstanceId> {
        match self {
            PhysicalComponentState::Planned => None,
            PhysicalComponentState::Existing(inner) => Some(inner.id()),
            PhysicalComponentState::StopPlanned(inner) => Some(inner.id()),
            PhysicalComponentState::Stopped(inner) => Some(inner.id()),
        }
    }
}

impl<C: PhysicalComponent> PhysicalComponentState<C> {
    fn new() -> Self {
        PhysicalComponentState::Planned
    }

    fn plan_stop(&mut self) -> Self {
        let old = std::mem::replace(self, PhysicalComponentState::Planned);
        match old {
            PhysicalComponentState::Existing(inner) => PhysicalComponentState::StopPlanned(inner),
            _ => {
                log::error!("Tried to mark bad function");
                old
            }
        }
    }

    fn mark_stopped(&mut self) -> Self {
        let old = std::mem::replace(self, PhysicalComponentState::Planned);
        match old {
            PhysicalComponentState::StopPlanned(inner) => PhysicalComponentState::Stopped(inner),
            _ => {
                log::error!("Tried to mark bad function");
                old
            }
        }
    }
}

pub trait PhysicalComponent {
    fn id(&self) -> edgeless_api::function_instance::InstanceId;
    fn creation_time(&self) -> std::time::Instant;
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

pub trait Node {
    fn node_id(&self) -> edgeless_api::function_instance::NodeId;
    fn cluster_id(&self) -> edgeless_api::function_instance::NodeId;
    fn available_runtimes(&self) -> Runtimes;
    fn available_resource_providers(&self) -> ResourceProviders;
    fn available_link_types(&self) -> LinkProviders;
    fn labels(&self) -> Vec<String>;
    fn is_proxy(&self) -> bool;
}

pub type Nodes<'a> = std::collections::HashMap<edgeless_api::function_instance::NodeId, &'a dyn Node>;

#[derive(Clone)]
pub enum Runtime<'a> {
    WasmBase(&'a dyn WasmRuntime),
    Native(&'a dyn NativeRuntime),
}

pub type Runtimes<'a> = std::collections::HashMap<String, Runtime<'a>>;

pub trait WasmRuntime {
    fn num_cores(&self) -> u32;
    fn cpu_freq_hz(&self) -> f32;
    fn mem_size_bytes(&self) -> u32;
    fn runtime_info(&self) -> Option<Box<dyn WasmRuntimeInfo>>;
}

pub trait NativeRuntime {
    fn num_cores(&self) -> u32;
    fn cpu_freq_hz(&self) -> f32;
    fn mem_size_bytes(&self) -> u32;
    #[allow(unused)]
    fn architecture(&self) -> NodeArchitecture;
    fn runtime_info(&self) -> Option<Box<dyn WasmRuntimeInfo>>;
}

pub enum NodeArchitecture {
    Amd64,
    Arm64,
    Xtensa,
}

pub trait WasmRuntimeInfo {
    fn cpu_load(&self) -> f32;
    fn mem_used(&self) -> f32;
    fn running_instances(&self) -> u32;
}

pub trait ResourceProvider {
    fn class_type(&self) -> String;
    // TODO(raphael) Update to use Ports.
    fn outputs(&self) -> Vec<String>;
}

pub type ResourceProviders<'a> = std::collections::HashMap<String, &'a dyn ResourceProvider>;

// Read-only view of a peer-cluster's state
pub trait Cluster {}

pub type Clusters<'a> = std::collections::HashMap<edgeless_api::function_instance::NodeId, &'a dyn Cluster>;

pub type LinkProviders = std::collections::HashMap<edgeless_api::link::LinkType, edgeless_api::link::LinkProviderId>;

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

        true
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

#[allow(clippy::large_enum_variant)]
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
    StopFunction {
        function_id: edgeless_api::function_instance::InstanceId,
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
