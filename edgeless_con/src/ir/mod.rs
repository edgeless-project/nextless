// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
// #![allow(unused)]

pub mod actor;
pub mod link;
pub mod managed_worflow;
pub mod pipeline;
pub mod proxy;
pub mod resource;
pub mod subflow;
pub mod support;
pub mod transformations;
pub mod workflow;

#[cfg(test)]
mod test;

pub trait LogicalComponent {
    fn logical_ports(&self) -> &LogicalPorts;
    fn logical_ports_mut(&mut self) -> &mut LogicalPorts;
    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId>;
    fn instances(&self) -> Vec<&std::cell::RefCell<PhysicalComponentState>>;
    fn split_view(&mut self) -> (&mut LogicalPorts, Vec<&std::cell::RefCell<PhysicalComponentState>>);
}

pub enum PhysicalComponentState {
    Invalid,
    Requested,
    Planned(Box<dyn PhysicalComponent>),
    Materialized(Box<dyn PhysicalComponent>),
    MigrationRequested(Box<dyn PhysicalComponent>),
    MigratingAway {
        old: Box<dyn PhysicalComponent>,
        new: edgeless_api::function_instance::InstanceId,
    },
    StopPlanned {
        old: Box<dyn PhysicalComponent>,
        replacement: Option<edgeless_api::function_instance::InstanceId>,
    },
    Stopped {
        dead_instance: Box<dyn PhysicalComponent>,
        #[allow(unused)]
        replacement: Option<edgeless_api::function_instance::InstanceId>,
    },
    #[allow(unused)]
    Dead(Box<dyn PhysicalComponent>),
    DeadReplaced {
        old: Box<dyn PhysicalComponent>,
        #[allow(unused)]
        replacement: edgeless_api::function_instance::InstanceId,
    },
    Lost(Box<dyn PhysicalComponent>),
    LostReplaced {
        old: Box<dyn PhysicalComponent>,
        #[allow(unused)]
        replacement: edgeless_api::function_instance::InstanceId,
    },
}

impl PhysicalComponentState {
    fn try_unpack_materialized(&self) -> Option<&dyn PhysicalComponent> {
        match self {
            PhysicalComponentState::Invalid => None,
            PhysicalComponentState::Requested => None,
            PhysicalComponentState::Planned(_) => None,
            PhysicalComponentState::Materialized(inner) => Some(inner.as_ref()),
            PhysicalComponentState::MigrationRequested(inner) => Some(inner.as_ref()),
            PhysicalComponentState::MigratingAway { old, .. } => Some(old.as_ref()),
            PhysicalComponentState::StopPlanned { old, .. } => Some(old.as_ref()),
            PhysicalComponentState::Stopped { dead_instance, .. } => Some(dead_instance.as_ref()),
            PhysicalComponentState::Dead(dead_instance) => Some(dead_instance.as_ref()),
            PhysicalComponentState::Lost(lost_instance) => Some(lost_instance.as_ref()),
            PhysicalComponentState::DeadReplaced { old, .. } => Some(old.as_ref()),
            PhysicalComponentState::LostReplaced { old, .. } => Some(old.as_ref()),
        }
    }

    fn try_unpack_active(&self) -> Option<&dyn PhysicalComponent> {
        match self {
            PhysicalComponentState::Invalid => None,
            PhysicalComponentState::Requested => None,
            PhysicalComponentState::Planned(_) => None,
            PhysicalComponentState::Materialized(inner) => Some(inner.as_ref()),
            PhysicalComponentState::MigrationRequested(inner) => Some(inner.as_ref()),
            PhysicalComponentState::MigratingAway { old, .. } => Some(old.as_ref()),
            PhysicalComponentState::StopPlanned { old, .. } => Some(old.as_ref()),
            PhysicalComponentState::Stopped { .. } => None,
            PhysicalComponentState::Dead(_) => None,
            PhysicalComponentState::Lost(_) => None,
            PhysicalComponentState::DeadReplaced { .. } => None,
            PhysicalComponentState::LostReplaced { .. } => None,
        }
    }

    fn try_unpack_materialized_mut(&mut self) -> Option<&mut dyn PhysicalComponent> {
        match self {
            PhysicalComponentState::Invalid => None,
            PhysicalComponentState::Requested => None,
            PhysicalComponentState::Planned(_) => None,
            PhysicalComponentState::Materialized(inner) => Some(inner.as_mut()),
            PhysicalComponentState::MigrationRequested(inner) => Some(inner.as_mut()),
            PhysicalComponentState::MigratingAway { old, .. } => Some(old.as_mut()),
            PhysicalComponentState::StopPlanned { old, .. } => Some(old.as_mut()),
            PhysicalComponentState::Stopped { dead_instance, .. } => Some(dead_instance.as_mut()),
            PhysicalComponentState::Dead(dead_instance) => Some(dead_instance.as_mut()),
            PhysicalComponentState::Lost(lost_instance) => Some(lost_instance.as_mut()),
            PhysicalComponentState::DeadReplaced { old, .. } => Some(old.as_mut()),
            PhysicalComponentState::LostReplaced { old, .. } => Some(old.as_mut()),
        }
    }

    fn id(&self) -> Option<edgeless_api::function_instance::InstanceId> {
        match self {
            PhysicalComponentState::Invalid => None,
            PhysicalComponentState::Requested => None,
            PhysicalComponentState::Planned(inner) => Some(inner.id()),
            PhysicalComponentState::Materialized(inner) => Some(inner.id()),
            PhysicalComponentState::MigrationRequested(inner) => Some(inner.id()),
            PhysicalComponentState::MigratingAway { old, .. } => Some(old.id()),
            PhysicalComponentState::StopPlanned { old, .. } => Some(old.id()),
            PhysicalComponentState::Stopped { dead_instance, .. } => Some(dead_instance.id()),
            PhysicalComponentState::Dead(dead_instance) => Some(dead_instance.id()),
            PhysicalComponentState::Lost(lost_instance) => Some(lost_instance.id()),
            PhysicalComponentState::DeadReplaced { old, .. } => Some(old.id()),
            PhysicalComponentState::LostReplaced { old, .. } => Some(old.id()),
        }
    }

    fn request_new_instance() -> Self {
        PhysicalComponentState::Requested
    }

    pub(crate) fn plan_creation(&mut self, instance: Box<dyn PhysicalComponent>) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Requested => PhysicalComponentState::Planned(instance),
            _ => {
                log::error!("Tried to plan creation of component in state other than 'requested'");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    pub(crate) fn mark_materialized(&mut self) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Planned(inner) => PhysicalComponentState::Materialized(inner),
            _ => {
                log::error!("Tried to mark function in state other than 'planned' as materialized");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    pub(crate) fn plan_stop(&mut self) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Materialized(inner) => PhysicalComponentState::StopPlanned {
                old: inner,
                replacement: None,
            },
            PhysicalComponentState::MigratingAway { old, new } => PhysicalComponentState::StopPlanned { old, replacement: Some(new) },
            _ => {
                log::error!("Tried to request stop of function that is not in a running state.");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }
    pub(crate) fn mark_migrating_away(&mut self, new_id: edgeless_api::function_instance::InstanceId) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Materialized(inner) => PhysicalComponentState::MigratingAway { old: inner, new: new_id },
            PhysicalComponentState::MigrationRequested(inner) => PhysicalComponentState::MigratingAway { old: inner, new: new_id },
            _ => {
                log::error!("Tried to mark function that is not currently running normaly as migrating.");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    fn mark_stopped(&mut self) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::StopPlanned { old, replacement } => PhysicalComponentState::Stopped {
                dead_instance: old,
                replacement,
            },
            _ => {
                log::error!("Tried to mark function in wrong state stopped");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    fn mark_lost(&mut self) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Materialized(inner) => PhysicalComponentState::Lost(inner),
            _ => {
                log::error!("Tried to mark non-active function as lost");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    fn mark_lost_replaced(&mut self, replacement: edgeless_api::function_instance::InstanceId) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Lost(old) => PhysicalComponentState::LostReplaced { old, replacement },
            _ => {
                log::error!("Tried to mark function that is not lost as lost_replaced");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    fn mark_dead_replaced(&mut self, replacement: edgeless_api::function_instance::InstanceId) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Dead(old) => PhysicalComponentState::DeadReplaced { old, replacement },
            _ => {
                log::error!("Tried to mark function that is not dead as dead_replaced");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    fn plan_migration(&mut self) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Materialized(inner) => PhysicalComponentState::MigrationRequested(inner),
            _ => {
                log::error!("Tried to migrate function in wrong state");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    fn abort_migration(&mut self) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::MigrationRequested(inner) => PhysicalComponentState::Materialized(inner),
            _ => {
                log::error!("Tried to abort migration on a component that is not in the migration state.");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }
}

pub trait PhysicalComponent: Send {
    fn id(&self) -> edgeless_api::function_instance::InstanceId;
    fn creation_time(&self) -> std::time::Instant;
    fn physical_ports(&mut self) -> &mut PhysicalPorts;
    fn materialize(&mut self, telemetry_provider: &Option<Box<dyn TelemetryProvider>>) -> Vec<RequiredChange>;
    fn stop(&mut self) -> Vec<RequiredChange>;
    fn materialized_state(&self) -> Option<&std::cell::RefCell<dyn MaterializedComponent>>;
    fn as_actor(&mut self) -> Option<&mut actor::PhysicalActor>;
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
    NativeBase(&'a dyn NativeRuntime),
}

impl Runtime<'_> {
    fn id(&self) -> String {
        match self {
            Runtime::WasmBase(_) => "WASM_BASE".to_string(),
            Runtime::NativeBase(_) => "NATIVE_BASE".to_string(),
        }
    }
}

pub type Runtimes<'a> = std::collections::HashMap<String, Runtime<'a>>;

pub trait WasmRuntime {
    fn num_cores(&self) -> u32;
    fn cpu_freq_hz(&self) -> f32;
    #[allow(unused)]
    fn mem_size_bytes(&self) -> u32;
    #[allow(unused)]
    fn runtime_info(&self) -> Option<Box<dyn WasmRuntimeInfo>>;
}

pub trait NativeRuntime {
    fn num_cores(&self) -> u32;
    fn cpu_freq_hz(&self) -> f32;
    #[allow(unused)]
    fn mem_size_bytes(&self) -> u32;
    fn node_architecture(&self) -> NodeArchitecture;
    #[allow(unused)]
    fn node_sys(&self) -> NodeSys;
    #[allow(unused)]
    fn runtime_info(&self) -> Option<Box<dyn WasmRuntimeInfo>>;
}

#[derive(PartialEq)]
pub enum NodeArchitecture {
    Amd64,
    Arm64,
    Xtensa,
}

#[derive(PartialEq)]
#[allow(unused)]
pub enum NodeSys {
    Linux,
    Darwin,
}

pub trait WasmRuntimeInfo {
    fn cpu_load(&self) -> f32;
    fn mem_used(&self) -> f32;
    fn running_instances(&self) -> u32;
}

pub trait ResourceProvider {
    fn class_type(&self) -> String;
    // TODO(raphael) Update to use Ports.
    #[allow(unused)]
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

#[derive(Clone, Default)]
pub struct PhysicalPorts {
    pub physical_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    pub physical_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
}

#[derive(Default)]
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

    fn update(
        &mut self,
        new: &PhysicalPorts,
        component_id: &edgeless_api::function_instance::InstanceId,
        telemetry_provider: &Option<Box<dyn TelemetryProvider>>,
    ) {
        self.materialized_inputs = new
            .physical_input_mapping
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    MaterializedInput {
                        mapping: v.clone(),
                        port_statistics: telemetry_provider.as_ref().map(|t| t.input_port_statistics_for(component_id, k)),
                    },
                )
            })
            .collect();
        self.materialized_outputs = new
            .physical_output_mapping
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    MaterializedOutput {
                        mapping: v.clone(),
                        port_statistics: telemetry_provider.as_ref().map(|t| t.output_port_statistics_for(component_id, k)),
                    },
                )
            })
            .collect();
    }
}

#[derive(Default)]
#[allow(unused)]
pub struct ExternalPorts {
    pub external_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
    pub external_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
}

#[derive(Default)]
#[allow(unused)]
pub struct InternalPorts {
    pub internal_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalOutput>,
    pub internal_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalInput>,
}

pub struct MaterializedInput {
    pub(crate) mapping: edgeless_api::common::Input,
    port_statistics: Option<Box<dyn PortStatistics>>,
}

impl MaterializedInput {
    #[allow(unused)]
    fn runtime_statistics(&self) -> Option<&dyn PortStatistics> {
        self.port_statistics.as_deref()
    }
}

pub struct MaterializedOutput {
    pub(crate) mapping: edgeless_api::common::Output,
    port_statistics: Option<Box<dyn PortStatistics>>,
}

impl MaterializedOutput {
    #[allow(unused)]
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

#[derive(Clone, Debug)]
pub enum LogicalInput {
    Direct(Vec<(String, edgeless_api::function_instance::PortId)>),
    Topic(String),
}

pub type LogicalOutput = edgeless_api::workflow_instance::PortMapping;

pub type PhysicalOutput = edgeless_api::common::Output;
pub type PhysicalInput = edgeless_api::common::Input;
