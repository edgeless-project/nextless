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

impl super::StatelessPhysicalTransformation for PhysicalConnectionMapper {
    #[tracing::instrument(name = "physical_mapper", skip_all)]
    fn apply(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
    ) -> Vec<super::PhysicalChange> {
        let mut required_changes = Vec::new();

        let components = targetable_component_instances(workflow);

        for component_id in components.keys() {
            let (component, component_instances) = workflow.get_component_with_instances(component_id).unwrap();

            let mut cloned_component_instances: std::collections::HashMap<_, _> =
                component_instances.map(|c| (c.component_id, (c.component.clone(), false))).collect();

            for (output_id, output) in &component.logical_ports().logical_output_mapping {
                let m = output.mapping.as_ref() as &dyn std::any::Any;
                let p = m.downcast_ref::<crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort>();

                let Some(logical_output) = p else {
                    tracing::warn!("Received unexpected dialect: {:?}. Normalization Failure?", output.dialect_type.base_type);
                    continue;
                };

                match &logical_output.destination {
                    interaction::dialect::logical_overlay::DestinationMapping::Unicast(logical_port_id) => {
                        map_unicast(
                            &mut cloned_component_instances,
                            output_id,
                            logical_port_id,
                            &components,
                            &workflow.cluster_id,
                        );
                    }
                    interaction::dialect::logical_overlay::DestinationMapping::Anycast(logical_port_ids) => {
                        map_anycast(
                            &mut cloned_component_instances,
                            output_id,
                            logical_port_ids,
                            &components,
                            &workflow.cluster_id,
                        );
                    }
                    interaction::dialect::logical_overlay::DestinationMapping::Multicast(logical_port_ids) => {
                        map_multicast(
                            &mut cloned_component_instances,
                            output_id,
                            logical_port_ids,
                            &components,
                            &workflow.cluster_id,
                        );
                    }
                }
            }

            for (component_id, (component, changed)) in cloned_component_instances {
                if !changed {
                    continue;
                }
                tracing::debug!("Mapping Changed");

                let component_update = crate::ir::transformations::PhysicalChange::Component(crate::ir::transformations::PhysicalComponentChange {
                    component_id,
                    action: crate::ir::transformations::PhysicalComponentChangeAction::Update(component),
                });

                required_changes.push(component_update);
            }
        }

        return required_changes;
    }
}

fn map_unicast(
    physical_instances: &mut std::collections::HashMap<uuid::Uuid, (crate::ir::PhysicalComponentState, bool)>,
    source_port_id: &edgeless_api::function_instance::PortId,
    logical_target_port_id: &interaction::LogicalPortId,
    component_instance_map: &std::collections::HashMap<String, Vec<edgeless_api::function_instance::InstanceId>>,
    cluster_id: &uuid::Uuid,
) {
    let target_component = &logical_target_port_id.component;
    let target_port_id = &logical_target_port_id.port;

    let target_instances = component_instance_map.get(target_component).unwrap().clone();

    for (_component_instance_id, (component, changed)) in physical_instances {
        if let Some(c_instance) = component.try_unpack_active_mut() {
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
                let old_mapping = c_instance.physical_ports_mut().physical_output_mapping.remove(&source_port_id);
                if old_mapping.is_some() {
                    *changed = true;
                }
                continue;
            };

            let mapping = Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(crate::ir::interaction::PhysicalPortId {
                    instance: target_instance_id,
                    port: target_port_id.clone(),
                }),
            });

            let new_mapping = crate::ir::interaction::SourcePortMapping {
                dialect_type: dialect_type(cluster_id),
                mapping: mapping,
            };

            let old_mapping = c_instance
                .physical_ports_mut()
                .physical_output_mapping
                .insert(source_port_id.clone(), new_mapping.clone());

            if let Some(old_mapping) = old_mapping {
                if old_mapping != new_mapping {
                    *changed = true;
                }
            } else {
                *changed = true;
            }
        }
    }
}

fn map_anycast<'a>(
    physical_instances: &mut std::collections::HashMap<uuid::Uuid, (crate::ir::PhysicalComponentState, bool)>,
    source_port_id: &edgeless_api::function_instance::PortId,
    logical_target_port_ids: impl IntoIterator<Item = &'a interaction::LogicalPortId>,
    component_instance_map: &std::collections::HashMap<String, Vec<edgeless_api::function_instance::InstanceId>>,
    cluster_id: &uuid::Uuid,
) {
    let mut target_instances = Vec::new();

    for logical_port_id in logical_target_port_ids {
        let target_id = &logical_port_id.component;
        let port_id = &logical_port_id.port;
        target_instances.append(
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

    for (_component_instance_id, (component, changed)) in physical_instances {
        if let Some(c_instance) = component.try_unpack_active_mut() {
            if target_instances.is_empty() {
                let old_mapping = c_instance.physical_ports_mut().physical_output_mapping.remove(&source_port_id);
                if old_mapping.is_some() {
                    *changed = true;
                }
                continue;
            }

            let new_mapping = crate::ir::interaction::SourcePortMapping {
                dialect_type: dialect_type(cluster_id),
                mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                    destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Anycast(
                        target_instances.iter().cloned().collect(),
                    ),
                }),
            };

            let old_mapping = c_instance
                .physical_ports_mut()
                .physical_output_mapping
                .insert(source_port_id.clone(), new_mapping.clone());
            if let Some(old_mapping) = old_mapping {
                if old_mapping != new_mapping {
                    *changed = true;
                }
            } else {
                *changed = true;
            }
        }
    }
}

fn map_multicast<'a>(
    physical_instances: &mut std::collections::HashMap<uuid::Uuid, (crate::ir::PhysicalComponentState, bool)>,
    source_port_id: &edgeless_api::function_instance::PortId,
    logical_target_port_ids: impl IntoIterator<Item = &'a interaction::LogicalPortId>,
    component_instance_map: &std::collections::HashMap<String, Vec<edgeless_api::function_instance::InstanceId>>,
    cluster_id: &uuid::Uuid,
) {
    let mut target_instances = Vec::new();
    for logical_port_id in logical_target_port_ids {
        let target_id = &logical_port_id.component;
        let port_id = &logical_port_id.port;
        target_instances.append(
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
    for (_component_instance_id, (component, changed)) in physical_instances {
        if let Some(c_instance) = component.try_unpack_active_mut() {
            if target_instances.len() == 0 {
                let old_mapping = c_instance.physical_ports_mut().physical_output_mapping.remove(&source_port_id);
                if old_mapping.is_some() {
                    *changed = true;
                }
                continue;
            }

            let new_mapping = crate::ir::interaction::SourcePortMapping {
                dialect_type: dialect_type(cluster_id),
                mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                    destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Multicast(
                        target_instances.iter().cloned().collect(),
                    ),
                }),
            };

            let old_mapping = c_instance
                .physical_ports_mut()
                .physical_output_mapping
                .insert(source_port_id.clone(), new_mapping.clone());

            if let Some(old_mapping) = old_mapping {
                if old_mapping != new_mapping {
                    *changed = true;
                }
            } else {
                *changed = true;
            }
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
        .components_with_instances()
        .into_iter()
        .map(|(id, _spec, instances)| {
            (
                id.to_string(),
                instances
                    .filter_map(|instance| match &*instance.component {
                        PhysicalComponentState::Planned(physical_component) => Some(physical_component.id()),
                        PhysicalComponentState::Materialized(physical_component) => Some(physical_component.id()),
                        _ => None,
                    })
                    .collect(),
            )
        })
        .collect::<std::collections::HashMap<String, Vec<edgeless_api::function_instance::InstanceId>>>()
}

#[cfg(test)]
mod test {
    use edgeless_api::function_instance::PortId;

    use crate::ir::transformations::StatelessPhysicalTransformation;

    #[test]
    fn unicast_mapping_single_target() {
        let example_node_id = uuid::Uuid::new_v4();

        let mock_node = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id.clone())
            .build()
            .unwrap();

        let nodes = std::collections::HashMap::from([(example_node_id.clone(), &mock_node as &dyn crate::ir::Node)]);

        let source_port_id = PortId("output1".to_string());
        let destination_port_id = PortId("port_1".to_string());

        let (actor_a_id, actor_a) = crate::ir::actor::mock_actor::MockActorBuilder::default()
            .with_logical_id("actor_under_test".to_string())
            .with_logical_ports(crate::ir::LogicalPorts {
                logical_output_mapping: std::collections::HashMap::from([(
                    source_port_id.clone(),
                    crate::ir::interaction::SourcePortMapping {
                        dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                            base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                            constraints: Default::default(),
                        },
                        mapping: Box::new(crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort {
                            destination: crate::ir::interaction::dialect::logical_overlay::DestinationMapping::Unicast(
                                crate::ir::interaction::LogicalPortId {
                                    component: "other_component".to_string(),
                                    port: destination_port_id.clone(),
                                },
                            ),
                        }),
                    },
                )]),
                logical_input_mapping: Default::default(),
            })
            .build();

        let (_, instance_a_id, instance_a) = crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&actor_a_id, &actor_a)
            .with_node_id(example_node_id)
            .build();

        let (actor_b_id, actor_b) = crate::ir::actor::mock_actor::MockActorBuilder::default()
            .with_logical_id("other_component".to_string())
            .with_logical_ports(crate::ir::LogicalPorts {
                logical_output_mapping: Default::default(),
                logical_input_mapping: std::collections::HashMap::from([(
                    destination_port_id.clone(),
                    crate::ir::interaction::DestiantionPortMapping {
                        dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                            base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                            constraints: Default::default(),
                        },
                        mapping: Box::new(crate::ir::interaction::dialect::logical_overlay::LogicalOverlayDestinationPort {
                            sources: std::collections::BTreeSet::from([crate::ir::interaction::LogicalPortId {
                                component: actor_a_id.clone(),
                                port: source_port_id.clone(),
                            }]),
                        }),
                    },
                )]),
            })
            .build();

        let (_, instance_b_id, instance_b) = crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&actor_b_id, &actor_b)
            .with_state(crate::ir::actor::mock_actor::DesiredPhysicalComponentState::Planned)
            .with_node_id(example_node_id)
            .build();

        let mut workflow = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&actor_a_id, &actor_a, &[(instance_a_id, instance_a)])
            .with_component(&actor_b_id, &actor_b, &[(instance_b_id, instance_b.clone())])
            .build();

        let mut mapper = super::PhysicalConnectionMapper::new();
        let changes = mapper.apply(&mut workflow, &nodes, &crate::ir::Clusters::default());

        assert_eq!(changes.len(), 1);

        let change = changes[0].clone();

        let crate::ir::transformations::PhysicalChange::Component(component_change) = change else {
            panic!("Not the expected change");
        };

        assert_eq!(component_change.component_id, instance_a_id.clone());

        let crate::ir::transformations::PhysicalComponentChangeAction::Update(instance) = component_change.action else {
            panic!("Not the expected change");
        };

        let instance = instance.try_unpack_active().unwrap();

        assert_eq!(instance.physical_ports().physical_input_mapping.len(), 0);
        assert_eq!(instance.physical_ports().physical_output_mapping.len(), 1);

        let mapping = instance.physical_ports().physical_output_mapping.get(&source_port_id).unwrap();

        assert_eq!(mapping.dialect_type.base_type, crate::ir::interaction::dialect::physical_overlay::ID);
        assert_eq!(mapping.dialect_type.constraints.len(), 1);

        use crate::ir::interaction::dialect::AsConcreteSourcePort;
        let physical_mapping =
            crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort::as_concrete(mapping.mapping.as_ref()).unwrap();

        assert_eq!(
            physical_mapping.destination,
            crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(crate::ir::interaction::PhysicalPortId {
                instance: instance_b.id().unwrap(),
                port: destination_port_id,
            })
        )
    }

    #[test]
    fn unicast_mapping_closer_node() {}

    #[test]
    fn unicast_mapping_random_node() {}

    #[test]
    fn anycast_mapping() {}

    #[test]
    fn multicast_mapping() {}
}
