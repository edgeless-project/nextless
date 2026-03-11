// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub struct PhysicalInstance<'a> {
    pub component_id: uuid::Uuid,
    pub component: &'a PhysicalComponentState,
}

#[derive(Clone)]
pub enum PhysicalComponentState {
    Invalid,
    Requested(Option<crate::ir::component::NodeFilters>),
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

pub trait PhysicalComponent: Send + PhysicalComponentClone {
    fn id(&self) -> edgeless_api::function_instance::InstanceId;
    fn creation_time(&self) -> std::time::Instant;
    fn physical_ports(&self) -> &PhysicalPorts;
    fn physical_ports_mut(&mut self) -> &mut PhysicalPorts;
    fn materialize(
        &self,
        telemetry_provider: &Option<Box<dyn super::TelemetryProvider>>,
    ) -> (Vec<crate::ir::transformations::PhysicalChange>, Vec<super::RequiredChange>);
    fn stop(&self) -> Vec<super::RequiredChange>;
    fn materialized_state(&self) -> Option<&dyn MaterializedComponent>;
    fn as_actor(&self) -> Option<&super::actor::PhysicalActor>;
    fn as_actor_mut(&mut self) -> Option<&mut super::actor::PhysicalActor>;
    fn as_resource(&self) -> Option<&super::resource::PhysicalResource>;
    fn as_resource_mut(&mut self) -> Option<&mut super::resource::PhysicalResource>;
    fn logical_parent(&self) -> String;
}

// https://stackoverflow.com/a/30353928
pub trait PhysicalComponentClone {
    fn clone_box(&self) -> Box<dyn PhysicalComponent>;
}

impl<T> PhysicalComponentClone for T
where
    T: 'static + PhysicalComponent + Clone,
{
    fn clone_box(&self) -> Box<dyn PhysicalComponent> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn PhysicalComponent> {
    fn clone(&self) -> Box<dyn PhysicalComponent> {
        self.clone_box()
    }
}

pub trait MaterializedComponent {
    fn materialized_ports(&self) -> &MaterializedPorts;
    fn runtime_statistics(&self) -> Option<&dyn ComponentRuntimeStatistics>;
}

pub trait ComponentRuntimeStatistics: Sync + Send + ComponentRuntimeStatisticsClone {
    fn invocation_rate_abs(&self, period: std::time::Duration) -> Option<f64>;
    fn invocations_rate_abs_by_port(&self, period: std::time::Duration) -> Vec<(edgeless_api::function_instance::PortId, f64)>;
    fn duration_mean_secs(&self, period: std::time::Duration) -> Option<f64>;
    fn duration_soft_limit_rate_rel(&self, period: std::time::Duration) -> Option<f64>;
    fn error_rate_rel(&self, period: std::time::Duration) -> Option<f64>;

    // TODO(raphael) Add Memory Tracking
    // fn memory_top_mean_bytes(&self, period: std::time::Duration) -> Option<f64>;
    // fn memory_soft_limit_rate_rel(&self, period: std::time::Duration) -> Option<f64>;
}

// https://stackoverflow.com/a/30353928
pub trait ComponentRuntimeStatisticsClone {
    fn clone_box(&self) -> Box<dyn ComponentRuntimeStatistics>;
}

impl<T> ComponentRuntimeStatisticsClone for T
where
    T: 'static + ComponentRuntimeStatistics + Clone,
{
    fn clone_box(&self) -> Box<dyn ComponentRuntimeStatistics> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn ComponentRuntimeStatistics> {
    fn clone(&self) -> Box<dyn ComponentRuntimeStatistics> {
        self.clone_box()
    }
}

pub trait PortStatistics: Sync + Send + PortStatisticsClone {
    fn message_rate_abs(&self, period: std::time::Duration) -> Option<f64>;
    fn message_rate_abs_by_peer(&self, period: std::time::Duration) -> Vec<(edgeless_api::function_instance::InstanceId, f64)>;
    fn message_size_mean_bytes(&self, period: std::time::Duration) -> Option<f64>;
    fn message_size_mean_byte_by_peer(&self, period: std::time::Duration) -> Vec<(edgeless_api::function_instance::InstanceId, f64)>;

    // TODO: Probably no great way to meaure this (one-way latencies)
    // fn latency_mean_secs(&self, period: Option<std::time::Duration>) -> f64;
    // fn latency_by_peer_mean_secs(&self, period: Option<std::time::Duration>) -> Vec<(edgeless_api::function_instance::InstanceId, f64)>;
}

// https://stackoverflow.com/a/30353928
pub trait PortStatisticsClone {
    fn clone_box(&self) -> Box<dyn PortStatistics>;
}

impl<T> PortStatisticsClone for T
where
    T: 'static + PortStatistics + Clone,
{
    fn clone_box(&self) -> Box<dyn PortStatistics> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn PortStatistics> {
    fn clone(&self) -> Box<dyn PortStatistics> {
        self.clone_box()
    }
}

pub type PhysicalOutput = crate::ir::interaction::SourcePortMapping;
pub type PhysicalInput = crate::ir::interaction::DestiantionPortMapping;

#[allow(unused)]
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

#[allow(unused)]
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

#[derive(Default, Clone)]
pub struct MaterializedPorts {
    pub materialized_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, MaterializedOutput>,
    pub materialized_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, MaterializedInput>,
}

#[derive(Clone)]
pub struct MaterializedInput {
    pub(crate) mapping: PhysicalInput,
    pub(crate) port_statistics: Option<Box<dyn PortStatistics>>,
}

#[derive(Clone)]
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
            PhysicalComponentState::Requested(_) => None,
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

    pub fn try_unpack_active<'a>(&'a self) -> Option<&'a dyn PhysicalComponent> {
        match self {
            PhysicalComponentState::Invalid => None,
            PhysicalComponentState::Requested(_) => None,
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
            PhysicalComponentState::Requested(_) => None,
            PhysicalComponentState::Planned(inner) => Some(inner.as_mut()),
            PhysicalComponentState::Materialized(inner) => Some(inner.as_mut()),
            PhysicalComponentState::MigrationRequested(inner) => Some(inner.as_mut()),
            PhysicalComponentState::MigratingAway { old, .. } => Some(old.as_mut()),
            PhysicalComponentState::StopPlanned { old, .. } => Some(old.as_mut()),
            PhysicalComponentState::Stopped { .. } => None,
            PhysicalComponentState::Dead(old) => Some(old.as_mut()),
            PhysicalComponentState::Lost(old) => Some(old.as_mut()),
            PhysicalComponentState::DeadReplaced { .. } => None,
            PhysicalComponentState::LostReplaced { .. } => None,
        }
    }

    pub fn try_unpack_materialized_mut(&mut self) -> Option<&mut dyn PhysicalComponent> {
        match self {
            PhysicalComponentState::Invalid => None,
            PhysicalComponentState::Requested(_) => None,
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
            PhysicalComponentState::Requested(_) => None,
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

    pub(crate) fn request_new_instance() -> (uuid::Uuid, Self) {
        (uuid::Uuid::new_v4(), PhysicalComponentState::Requested(None))
    }

    pub(crate) fn request_new_instance_with_extra_constraints(extra_constraints: crate::ir::component::NodeFilters) -> (uuid::Uuid, Self) {
        (uuid::Uuid::new_v4(), PhysicalComponentState::Requested(Some(extra_constraints)))
    }

    pub(crate) fn plan_creation(&self, instance: Box<dyn PhysicalComponent>) -> Option<PhysicalComponentState> {
        match self {
            PhysicalComponentState::Requested(_) => Some(PhysicalComponentState::Planned(instance)),
            _ => {
                tracing::error!("Tried to plan creation of component in state other than 'requested'");
                None
            }
        }
    }

    pub fn logical_component_id(&self) -> Option<String> {
        if let Some(c) = self.try_unpack_active() {
            return Some(c.logical_parent());
        }
        None
    }

    pub(crate) fn plan_stop(&self) -> Option<PhysicalComponentState> {
        match self {
            PhysicalComponentState::Planned(instance) => Some(PhysicalComponentState::Stopped {
                dead_instance: instance.clone(),
                replacement: None,
            }),
            PhysicalComponentState::Materialized(instance) => Some(PhysicalComponentState::StopPlanned {
                old: instance.clone(),
                replacement: None,
            }),
            PhysicalComponentState::MigrationRequested(instance) => Some(PhysicalComponentState::StopPlanned {
                old: instance.clone(),
                replacement: None,
            }),
            PhysicalComponentState::MigratingAway { old, new } => Some(PhysicalComponentState::StopPlanned {
                old: old.clone(),
                replacement: Some(new.clone()),
            }),
            PhysicalComponentState::StopPlanned { old, replacement } => Some(PhysicalComponentState::StopPlanned {
                old: old.clone(),
                replacement: replacement.clone(),
            }),
            PhysicalComponentState::Dead(old) => Some(PhysicalComponentState::Stopped {
                dead_instance: old.clone(),
                replacement: None,
            }),
            PhysicalComponentState::Lost(old) => Some(PhysicalComponentState::Stopped {
                dead_instance: old.clone(),
                replacement: None,
            }),
            _ => {
                tracing::error!("Tried to request stop of function that is not in a running state.");
                None
            }
        }
    }
    pub(crate) fn mark_migrating_away(&self, new_id: edgeless_api::function_instance::InstanceId) -> Option<PhysicalComponentState> {
        match self {
            PhysicalComponentState::Materialized(inner) => Some(PhysicalComponentState::MigratingAway {
                old: inner.clone(),
                new: new_id,
            }),
            PhysicalComponentState::MigrationRequested(inner) => Some(PhysicalComponentState::MigratingAway {
                old: inner.clone(),
                new: new_id,
            }),
            _ => {
                tracing::error!("Tried to mark function that is not currently running normaly as migrating.");
                None
            }
        }
    }

    pub(crate) fn mark_stopped(&self) -> Option<PhysicalComponentState> {
        match self {
            PhysicalComponentState::Planned(instance) => Some(PhysicalComponentState::Stopped {
                dead_instance: instance.clone(),
                replacement: None,
            }),
            PhysicalComponentState::StopPlanned { old, replacement } => Some(PhysicalComponentState::Stopped {
                dead_instance: old.clone(),
                replacement: replacement.clone(),
            }),
            PhysicalComponentState::Dead(old) => Some(PhysicalComponentState::Stopped {
                dead_instance: old.clone(),
                replacement: None,
            }),
            PhysicalComponentState::Lost(old) => Some(PhysicalComponentState::Stopped {
                dead_instance: old.clone(),
                replacement: None,
            }),
            _ => {
                tracing::error!("Tried to mark function in wrong state stopped");
                None
            }
        }
    }

    pub(crate) fn mark_lost(&self) -> Option<PhysicalComponentState> {
        match self {
            PhysicalComponentState::Materialized(inner) => Some(PhysicalComponentState::Lost(inner.clone())),
            _ => {
                tracing::error!("Tried to mark non-active function as lost");
                None
            }
        }
    }

    pub(crate) fn mark_lost_replaced(&self, replacement: edgeless_api::function_instance::InstanceId) -> Option<PhysicalComponentState> {
        match self {
            PhysicalComponentState::Lost(old) => Some(PhysicalComponentState::LostReplaced {
                old: old.clone(),
                replacement,
            }),
            _ => {
                tracing::error!("Tried to mark function that is not lost as lost_replaced");
                None
            }
        }
    }

    pub(crate) fn mark_dead_replaced(&self, replacement: edgeless_api::function_instance::InstanceId) -> Option<PhysicalComponentState> {
        match self {
            PhysicalComponentState::Dead(old) => Some(PhysicalComponentState::DeadReplaced {
                old: old.clone(),
                replacement,
            }),
            _ => {
                tracing::error!("Tried to mark function that is not dead as dead_replaced");
                None
            }
        }
    }

    pub(crate) fn plan_migration(&self) -> Option<PhysicalComponentState> {
        match self {
            PhysicalComponentState::Materialized(inner) => Some(PhysicalComponentState::MigrationRequested(inner.clone())),
            _ => {
                tracing::error!("Tried to migrate function in wrong state");
                None
            }
        }
    }

    pub(crate) fn abort_migration(&self) -> Option<PhysicalComponentState> {
        match self {
            PhysicalComponentState::MigrationRequested(inner) => Some(PhysicalComponentState::Materialized(inner.clone())),
            _ => {
                tracing::error!("Tried to abort migration on a component that is not in the migration state.");
                None
            }
        }
    }
}

impl<'a> PhysicalInstance<'a> {
    pub(crate) fn abort_migration(&self) -> Vec<crate::ir::transformations::PhysicalChange> {
        match self.component.abort_migration() {
            Some(new_component) => vec![crate::ir::transformations::PhysicalChange::Component(
                crate::ir::transformations::PhysicalComponentChange {
                    component_id: self.component_id.clone(),
                    action: crate::ir::transformations::PhysicalComponentChangeAction::Update(new_component),
                },
            )],
            None => vec![],
        }
    }

    pub(crate) fn mark_migrating_away(&self, new_id: edgeless_api::function_instance::InstanceId) -> Vec<crate::ir::transformations::PhysicalChange> {
        match self.component.mark_migrating_away(new_id) {
            Some(new_component) => vec![crate::ir::transformations::PhysicalChange::Component(
                crate::ir::transformations::PhysicalComponentChange {
                    component_id: self.component_id.clone(),
                    action: crate::ir::transformations::PhysicalComponentChangeAction::Update(new_component),
                },
            )],
            None => vec![],
        }
    }

    pub(crate) fn mark_lost(&self) -> Vec<crate::ir::transformations::PhysicalChange> {
        match self.component.mark_lost() {
            Some(new_component) => vec![crate::ir::transformations::PhysicalChange::Component(
                crate::ir::transformations::PhysicalComponentChange {
                    component_id: self.component_id.clone(),
                    action: crate::ir::transformations::PhysicalComponentChangeAction::Update(new_component),
                },
            )],
            None => vec![],
        }
    }

    pub(crate) fn mark_lost_replaced(&self, new_id: edgeless_api::function_instance::InstanceId) -> Vec<crate::ir::transformations::PhysicalChange> {
        match self.component.mark_lost_replaced(new_id) {
            Some(new_component) => vec![crate::ir::transformations::PhysicalChange::Component(
                crate::ir::transformations::PhysicalComponentChange {
                    component_id: self.component_id.clone(),
                    action: crate::ir::transformations::PhysicalComponentChangeAction::Update(new_component),
                },
            )],
            None => vec![],
        }
    }

    pub(crate) fn mark_dead_replaced(&self, new_id: edgeless_api::function_instance::InstanceId) -> Vec<crate::ir::transformations::PhysicalChange> {
        match self.component.mark_dead_replaced(new_id) {
            Some(new_component) => vec![crate::ir::transformations::PhysicalChange::Component(
                crate::ir::transformations::PhysicalComponentChange {
                    component_id: self.component_id.clone(),
                    action: crate::ir::transformations::PhysicalComponentChangeAction::Update(new_component),
                },
            )],
            None => vec![],
        }
    }

    pub(crate) fn mark_stopped(&self) -> Vec<crate::ir::transformations::PhysicalChange> {
        match self.component.mark_stopped() {
            Some(new_component) => vec![crate::ir::transformations::PhysicalChange::Component(
                crate::ir::transformations::PhysicalComponentChange {
                    component_id: self.component_id.clone(),
                    action: crate::ir::transformations::PhysicalComponentChangeAction::Update(new_component),
                },
            )],
            None => vec![],
        }
    }

    pub(crate) fn plan_stop(&self) -> Vec<crate::ir::transformations::PhysicalChange> {
        match self.component.plan_stop() {
            Some(new_component) => vec![crate::ir::transformations::PhysicalChange::Component(
                crate::ir::transformations::PhysicalComponentChange {
                    component_id: self.component_id.clone(),
                    action: crate::ir::transformations::PhysicalComponentChangeAction::Update(new_component),
                },
            )],
            None => vec![],
        }
    }

    pub(crate) fn plan_migration(&self) -> Vec<crate::ir::transformations::PhysicalChange> {
        match self.component.plan_migration() {
            Some(new_component) => vec![crate::ir::transformations::PhysicalChange::Component(
                crate::ir::transformations::PhysicalComponentChange {
                    component_id: self.component_id.clone(),
                    action: crate::ir::transformations::PhysicalComponentChangeAction::Update(new_component),
                },
            )],
            None => vec![],
        }
    }
}
