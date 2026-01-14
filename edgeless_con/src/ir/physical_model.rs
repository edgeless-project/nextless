// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

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

pub trait PhysicalComponent: Send {
    fn id(&self) -> edgeless_api::function_instance::InstanceId;
    fn creation_time(&self) -> std::time::Instant;
    fn physical_ports(&mut self) -> &mut PhysicalPorts;
    fn materialize(&mut self, telemetry_provider: &Option<Box<dyn super::TelemetryProvider>>) -> Vec<super::RequiredChange>;
    fn stop(&mut self) -> Vec<super::RequiredChange>;
    fn materialized_state(&self) -> Option<&std::cell::RefCell<dyn MaterializedComponent>>;
    fn as_actor(&mut self) -> Option<&mut super::actor::PhysicalActor>;
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

pub type PhysicalOutput = crate::ir::interaction::SourcePortMapping;
pub type PhysicalInput = crate::ir::interaction::DestiantionPortMapping;

pub fn parse_api_output_mapping(
    mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Output>,
) -> std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::SourcePortMapping> {
    mapping.into_iter().map(|(port_id, port)| (port_id, port.into())).collect()
}

impl From<edgeless_api::common::Output> for PhysicalOutput {
    fn from(value: edgeless_api::common::Output) -> Self {
        match value {
            edgeless_api::common::Output::Single(instance_id, port_id) => crate::ir::interaction::SourcePortMapping {
                dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                    base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                    constraints: std::collections::BTreeSet::new(),
                },
                mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                    destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(
                        crate::ir::interaction::PhysicalPortId {
                            instance: instance_id,
                            port: port_id,
                        },
                    ),
                }),
            },
            edgeless_api::common::Output::Any(items) => crate::ir::interaction::SourcePortMapping {
                dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                    base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                    constraints: std::collections::BTreeSet::new(),
                },
                mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                    destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Anycast(
                        items
                            .into_iter()
                            .map(|i| crate::ir::interaction::PhysicalPortId { instance: i.0, port: i.1 })
                            .collect(),
                    ),
                }),
            },
            edgeless_api::common::Output::All(items) => crate::ir::interaction::SourcePortMapping {
                dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                    base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                    constraints: std::collections::BTreeSet::new(),
                },
                mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                    destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Multicast(
                        items
                            .into_iter()
                            .map(|i| crate::ir::interaction::PhysicalPortId { instance: i.0, port: i.1 })
                            .collect(),
                    ),
                }),
            },
            edgeless_api::common::Output::Link(_link_instance_id) => {
                panic!("Link Parsing not Implemented Yet.")
            }
        }
    }
}

pub fn parse_api_input_mapping(
    mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Input>,
) -> std::collections::HashMap<edgeless_api::function_instance::PortId, crate::ir::interaction::DestiantionPortMapping> {
    mapping
        .into_iter()
        .filter_map(|(port_id, port)| match port.try_into() {
            Ok(port) => Some((port_id, port)),
            Err(_) => None,
        })
        .collect()
}

impl TryFrom<edgeless_api::common::Input> for PhysicalInput {
    type Error = anyhow::Error;

    fn try_from(value: edgeless_api::common::Input) -> Result<Self, Self::Error> {
        match value {
            edgeless_api::common::Input::Stub => {
                panic!("Handling Stubs not Implemented Yet.")
            }
            edgeless_api::common::Input::Link(_link_instance_id) => {
                panic!("Link Parsing not Implemented Yet.")
            }
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct PhysicalPorts {
    pub physical_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    pub physical_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
}

#[derive(Default)]
pub struct MaterializedPorts {
    pub materialized_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, MaterializedOutput>,
    pub materialized_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, MaterializedInput>,
}

pub struct MaterializedInput {
    pub(crate) mapping: PhysicalInput,
    pub(crate) port_statistics: Option<Box<dyn PortStatistics>>,
}

pub struct MaterializedOutput {
    pub(crate) mapping: PhysicalOutput,
    pub(crate) port_statistics: Option<Box<dyn PortStatistics>>,
}

impl MaterializedInput {
    #[allow(unused)]
    fn runtime_statistics(&self) -> Option<&dyn PortStatistics> {
        self.port_statistics.as_deref()
    }
}

impl std::fmt::Debug for MaterializedOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.mapping.fmt(f)
    }
}

impl MaterializedOutput {
    #[allow(unused)]
    fn runtime_statistics(&self) -> Option<&dyn PortStatistics> {
        self.port_statistics.as_deref()
    }
}

impl MaterializedPorts {
    pub(crate) fn is_current_mapping(&self, other: &PhysicalPorts) -> bool {
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

    pub(crate) fn update(
        &mut self,
        new: &PhysicalPorts,
        component_id: &edgeless_api::function_instance::InstanceId,
        telemetry_provider: &Option<Box<dyn super::TelemetryProvider>>,
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

impl PhysicalComponentState {
    pub fn try_unpack_materialized(&self) -> Option<&dyn PhysicalComponent> {
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

    pub fn try_unpack_active(&self) -> Option<&dyn PhysicalComponent> {
        match self {
            PhysicalComponentState::Invalid => None,
            PhysicalComponentState::Requested => None,
            PhysicalComponentState::Planned(inner) => Some(inner.as_ref()),
            PhysicalComponentState::Materialized(inner) => Some(inner.as_ref()),
            PhysicalComponentState::MigrationRequested(inner) => Some(inner.as_ref()),
            PhysicalComponentState::MigratingAway { old, .. } => Some(old.as_ref()),
            PhysicalComponentState::StopPlanned { old, .. } => Some(old.as_ref()),
            PhysicalComponentState::Stopped { .. } => None,
            PhysicalComponentState::Dead(old) => Some(old.as_ref()),
            PhysicalComponentState::Lost(old) => Some(old.as_ref()),
            PhysicalComponentState::DeadReplaced { .. } => None,
            PhysicalComponentState::LostReplaced { .. } => None,
        }
    }

    pub fn try_unpack_active_mut(&mut self) -> Option<&mut dyn PhysicalComponent> {
        match self {
            PhysicalComponentState::Invalid => None,
            PhysicalComponentState::Requested => None,
            PhysicalComponentState::Planned(inner) => Some(inner.as_mut()),
            PhysicalComponentState::Materialized(inner) => Some(inner.as_mut()),
            PhysicalComponentState::MigrationRequested(inner) => Some(inner.as_mut()),
            PhysicalComponentState::MigratingAway { old, .. } => Some(old.as_mut()),
            PhysicalComponentState::StopPlanned { old, .. } => Some(old.as_mut()),
            PhysicalComponentState::Stopped { .. } => None,
            PhysicalComponentState::Dead(_) => None,
            PhysicalComponentState::Lost(_) => None,
            PhysicalComponentState::DeadReplaced { .. } => None,
            PhysicalComponentState::LostReplaced { .. } => None,
        }
    }

    pub fn try_unpack_materialized_mut(&mut self) -> Option<&mut dyn PhysicalComponent> {
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

    pub fn id(&self) -> Option<edgeless_api::function_instance::InstanceId> {
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

    pub(crate) fn request_new_instance() -> Self {
        PhysicalComponentState::Requested
    }

    pub(crate) fn plan_creation(&mut self, instance: Box<dyn PhysicalComponent>) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Requested => PhysicalComponentState::Planned(instance),
            _ => {
                tracing::error!("Tried to plan creation of component in state other than 'requested'");
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
                tracing::error!("Tried to mark function in state other than 'planned' as materialized");
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
                tracing::error!("Tried to request stop of function that is not in a running state.");
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
                tracing::error!("Tried to mark function that is not currently running normaly as migrating.");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    pub(crate) fn mark_stopped(&mut self) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::StopPlanned { old, replacement } => PhysicalComponentState::Stopped {
                dead_instance: old,
                replacement,
            },
            _ => {
                tracing::error!("Tried to mark function in wrong state stopped");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    pub(crate) fn mark_lost(&mut self) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Materialized(inner) => PhysicalComponentState::Lost(inner),
            _ => {
                tracing::error!("Tried to mark non-active function as lost");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    pub(crate) fn mark_lost_replaced(&mut self, replacement: edgeless_api::function_instance::InstanceId) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Lost(old) => PhysicalComponentState::LostReplaced { old, replacement },
            _ => {
                tracing::error!("Tried to mark function that is not lost as lost_replaced");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    pub(crate) fn mark_dead_replaced(&mut self, replacement: edgeless_api::function_instance::InstanceId) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Dead(old) => PhysicalComponentState::DeadReplaced { old, replacement },
            _ => {
                tracing::error!("Tried to mark function that is not dead as dead_replaced");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    pub(crate) fn plan_migration(&mut self) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::Materialized(inner) => PhysicalComponentState::MigrationRequested(inner),
            _ => {
                tracing::error!("Tried to migrate function in wrong state");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }

    pub(crate) fn abort_migration(&mut self) {
        let old = std::mem::replace(self, PhysicalComponentState::Invalid);
        let new = match old {
            PhysicalComponentState::MigrationRequested(inner) => PhysicalComponentState::Materialized(inner),
            _ => {
                tracing::error!("Tried to abort migration on a component that is not in the migration state.");
                old
            }
        };
        let _ = std::mem::replace(self, new);
    }
}
