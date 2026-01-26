// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::super::*;
use crate::ir::interaction::dialect::DialectConstraint;
use rand::seq::SliceRandom;

pub struct PhysicalConnectionMapper {}

impl PhysicalConnectionMapper {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatelessTransformation for PhysicalConnectionMapper {
    #[tracing::instrument(name = "physical_mapper", skip_all)]
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, _nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        let components = targetable_component_instances(workflow);

        for component_id in components.keys() {
            let mut component = workflow.get_component(component_id).unwrap().borrow_mut();
            let (logical_ports, physical_instances) = component.split_view();

            for (output_id, output) in &logical_ports.logical_output_mapping {
                let m = output.mapping.as_ref() as &dyn std::any::Any;
                let p = m.downcast_ref::<crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort>();

                let Some(logical_output) = p else {
                    tracing::warn!("Received unexpected dialect: {:?}. Normalization Failure?", output.dialect_type.base_type);
                    continue;
                };

                match &logical_output.destination {
                    interaction::dialect::logical_overlay::DestinationMapping::Unicast(logical_port_id) => {
                        map_unicast(&physical_instances, output_id, logical_port_id, &components, &workflow.cluster_id);
                    }
                    interaction::dialect::logical_overlay::DestinationMapping::Anycast(logical_port_ids) => {
                        map_anycast(&physical_instances, output_id, logical_port_ids, &components, &workflow.cluster_id);
                    }
                    interaction::dialect::logical_overlay::DestinationMapping::Multicast(logical_port_ids) => {
                        map_multicast(&physical_instances, output_id, logical_port_ids, &components, &workflow.cluster_id);
                    }
                }

                // We do not map the input ports here as they will be automatically generated from the source ports in the next step (physical_interaction_specializer).
                // If the next step required them to be normalized, we need to add a pass of port_to_interaction and interaction_to_port here.
            }
        }
    }
}

fn map_unicast(
    physical_instances: &Vec<&std::cell::RefCell<PhysicalComponentState>>,
    source_port_id: &edgeless_api::function_instance::PortId,
    logical_target_port_id: &interaction::LogicalPortId,
    component_instance_map: &std::collections::HashMap<String, Vec<edgeless_api::function_instance::InstanceId>>,
    cluster_id: &uuid::Uuid,
) {
    let target_component = &logical_target_port_id.component;
    let target_port_id = &logical_target_port_id.port;

    let target_instances = component_instance_map.get(target_component).unwrap().clone();

    for c_instance in physical_instances {
        if let Some(c_instance) = c_instance.borrow_mut().try_unpack_active_mut() {
            let target_instance = if target_instances.len() == 1 {
                Some(target_instances[0].clone())
            } else {
                tracing::debug!("Unicast Relationship with {} targets. Will sample one target.", target_instances.len());

                let best_targets = select_closest_target_instances(c_instance.id(), &target_instances);

                // https://stackoverflow.com/a/34215930
                best_targets.choose(&mut rand::thread_rng()).cloned()
            };

            let Some(target_instance_id) = target_instance else {
                tracing::warn!("Could not find phyiscal target instance.");
                continue;
            };

            let mapping = Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(crate::ir::interaction::PhysicalPortId {
                    instance: target_instance_id,
                    port: target_port_id.clone(),
                }),
            });

            c_instance.physical_ports().physical_output_mapping.insert(
                source_port_id.clone(),
                crate::ir::interaction::SourcePortMapping {
                    dialect_type: dialect_type(cluster_id),
                    mapping: mapping,
                },
            );
        }
    }
}

fn map_anycast<'a>(
    physical_instances: &Vec<&std::cell::RefCell<PhysicalComponentState>>,
    source_port_id: &edgeless_api::function_instance::PortId,
    logical_target_port_ids: impl IntoIterator<Item = &'a interaction::LogicalPortId>,
    component_instance_map: &std::collections::HashMap<String, Vec<edgeless_api::function_instance::InstanceId>>,
    cluster_id: &uuid::Uuid,
) {
    let mut instances = Vec::new();

    for logical_port_id in logical_target_port_ids {
        let target_id = &logical_port_id.component;
        let port_id = &logical_port_id.port;
        instances.append(
            &mut component_instance_map
                .get(target_id)
                .unwrap()
                .iter()
                .map(|target| crate::ir::interaction::PhysicalPortId {
                    instance: *target,
                    port: port_id.clone(),
                })
                .collect(),
        )
    }

    for c_instance in physical_instances {
        if let Some(c_instance) = c_instance.borrow_mut().try_unpack_active_mut() {
            c_instance.physical_ports().physical_output_mapping.insert(
                source_port_id.clone(),
                crate::ir::interaction::SourcePortMapping {
                    dialect_type: dialect_type(cluster_id),
                    mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                        destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Anycast(
                            instances.iter().cloned().collect(),
                        ),
                    }),
                },
            );
        }
    }
}

fn map_multicast<'a>(
    physical_instances: &Vec<&std::cell::RefCell<PhysicalComponentState>>,
    source_port_id: &edgeless_api::function_instance::PortId,
    logical_target_port_ids: impl IntoIterator<Item = &'a interaction::LogicalPortId>,
    component_instance_map: &std::collections::HashMap<String, Vec<edgeless_api::function_instance::InstanceId>>,
    cluster_id: &uuid::Uuid,
) {
    let mut instances = Vec::new();
    for logical_port_id in logical_target_port_ids {
        let target_id = &logical_port_id.component;
        let port_id = &logical_port_id.port;
        instances.append(
            &mut component_instance_map
                .get(target_id)
                .unwrap()
                .iter()
                .map(|target| crate::ir::interaction::PhysicalPortId {
                    instance: *target,
                    port: port_id.clone(),
                })
                .collect(),
        )
    }
    for c_instance in physical_instances {
        if let Some(c_instance) = c_instance.borrow_mut().try_unpack_active_mut() {
            c_instance.physical_ports().physical_output_mapping.insert(
                source_port_id.clone(),
                crate::ir::interaction::SourcePortMapping {
                    dialect_type: dialect_type(cluster_id),
                    mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                        destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Multicast(
                            instances.iter().cloned().collect(),
                        ),
                    }),
                },
            );
        }
    }
}

fn select_closest_target_instances(
    source_instance_id: edgeless_api::function_instance::InstanceId,
    target_instances: &[edgeless_api::function_instance::InstanceId],
) -> Vec<edgeless_api::function_instance::InstanceId> {
    let same_node_instances: Vec<_> = target_instances
        .iter()
        .filter(|target| target.node_id == source_instance_id.node_id)
        .cloned()
        .collect();

    if same_node_instances.len() > 0 {
        tracing::debug!("Limiting selection to local targets.");
        same_node_instances
    } else {
        tracing::debug!("No local targets, falling back to all available targets.");
        target_instances.to_vec()
    }
}

fn dialect_type(cluster_id: &uuid::Uuid) -> crate::ir::interaction::dialect::DialectDescriptor {
    crate::ir::interaction::dialect::DialectDescriptor {
        base_type: crate::ir::interaction::dialect::physical_overlay::ID,
        constraints: std::collections::BTreeSet::from([interaction::dialect::physical_overlay::PhysicalOverlayConstraint::Cluster(
            cluster_id.clone(),
        )
        .as_container()]),
    }
}

fn targetable_component_instances(
    workflow: &crate::ir::workflow::ActiveWorkflow,
) -> std::collections::HashMap<String, Vec<edgeless_api::function_instance::InstanceId>> {
    workflow
        .components()
        .into_iter()
        .map(|(id, spec)| {
            (
                id.to_string(),
                spec.borrow_mut()
                    .instances()
                    .iter()
                    .filter_map(|instance| {
                        let instance_borrow = instance.borrow();

                        match &*instance_borrow {
                            PhysicalComponentState::Planned(physical_component) => Some(physical_component.id()),
                            PhysicalComponentState::Materialized(physical_component) => Some(physical_component.id()),
                            _ => None,
                        }
                    })
                    .collect(),
            )
        })
        .collect::<std::collections::HashMap<String, Vec<edgeless_api::function_instance::InstanceId>>>()
}
