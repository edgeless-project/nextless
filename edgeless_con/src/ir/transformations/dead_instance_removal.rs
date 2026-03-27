// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use crate::ir::interaction::dialect::{AsConcreteDestinationPort, AsConcreteSourcePort};

pub use super::super::*;

pub struct DeadInstanceRemoval {}

struct ClonedComponentInstance {
    deletion_expected: bool,
    instance: crate::ir::physical_model::PhysicalComponentState,
    changed: bool,
}

impl DeadInstanceRemoval {
    pub fn new() -> Self {
        Self {}
    }
}

impl crate::ir::transformations::StatelessPhysicalTransformation for DeadInstanceRemoval {
    #[tracing::instrument(name = "dead_instance_removal", skip_all)]
    fn apply(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
    ) -> Vec<transformations::PhysicalChange> {
        let mut required_changes = Vec::new();

        let mut cloned_instances = std::collections::HashMap::<uuid::Uuid, ClonedComponentInstance>::new();

        for (_, _, component_instances) in workflow.components_with_instances() {
            for instance in component_instances {
                cloned_instances.insert(
                    instance.component_id.clone(),
                    ClonedComponentInstance {
                        deletion_expected: false,
                        instance: instance.component.clone(),
                        changed: false,
                    },
                );
            }
        }

        loop {
            let change = optimizer_iteration(workflow, &mut cloned_instances);
            if !change {
                break;
            }
        }

        for (instance_id, instance) in cloned_instances {
            if instance.changed {
                if instance.deletion_expected {
                    required_changes.extend(
                        PhysicalInstance {
                            component_id: instance_id.clone(),
                            component: &instance.instance,
                        }
                        .remove_planned_or_stop_running(),
                    )
                } else {
                    required_changes.push(crate::ir::transformations::PhysicalChange::Component(
                        crate::ir::transformations::PhysicalComponentChange {
                            component_id: instance_id,
                            action: crate::ir::transformations::PhysicalComponentChangeAction::Update(instance.instance),
                        },
                    ));
                }
            }
        }

        required_changes
    }
}

fn optimizer_iteration(
    workflow: &crate::ir::workflow::ActiveWorkflow,
    cloned_instances: &mut std::collections::HashMap<uuid::Uuid, ClonedComponentInstance>,
) -> bool {
    let mut change_in_this_iteration = false;
    let mut peer_source_port_entries_to_remove = Vec::new();
    let mut peer_destination_port_entries_to_remove = Vec::new();
    for (_component_id, logical_component, component_instances) in workflow.components_with_instances() {
        let crate::ir::LogicalComponent::Actor(logical_actor) = logical_component else {
            continue;
        };

        for original_component_instance in component_instances {
            let Some(component_instance) = cloned_instances.get_mut(&original_component_instance.component_id) else {
                tracing::warn!("No cloned instance for component instance.");
                continue;
            };
            if component_instance.deletion_expected {
                continue;
            }
            let Some(active_instance) = component_instance.instance.try_unpack_active_mut() else {
                continue;
            };
            // Input Handling
            {
                let (changed, instance_peer_source_ports_to_remove) = remove_unused_input_ports(logical_actor, active_instance);
                if changed {
                    component_instance.changed = true;
                    change_in_this_iteration = true;
                }
                peer_source_port_entries_to_remove.extend(instance_peer_source_ports_to_remove);
            }
            // Output Handling
            {
                let (changed, instance_peer_destination_ports_to_remove) = remove_unused_output_ports(logical_actor, active_instance);
                if changed {
                    component_instance.changed = true;
                    change_in_this_iteration = true;
                }
                peer_destination_port_entries_to_remove.extend(instance_peer_destination_ports_to_remove);
            }
            // Instance Handling
            {
                let (removal_requested, instance_peer_source_ports_to_remove, instance_peer_destination_ports_to_remove) =
                    remove_unused_instance(logical_actor, active_instance);

                if removal_requested {
                    component_instance.changed = true;
                    component_instance.deletion_expected = true;
                    change_in_this_iteration = true;
                }

                peer_source_port_entries_to_remove.extend(instance_peer_source_ports_to_remove);
                peer_destination_port_entries_to_remove.extend(instance_peer_destination_ports_to_remove);
            }
        }
    }
    let source_removal_change = remove_peer_source_ports(cloned_instances, &peer_source_port_entries_to_remove);
    let dest_removal_change = remove_peer_destination_ports(cloned_instances, &peer_destination_port_entries_to_remove);
    change_in_this_iteration = change_in_this_iteration || source_removal_change || dest_removal_change;

    change_in_this_iteration
}

fn remove_unused_input_ports(
    logical_actor: &crate::ir::actor::LogicalActor,
    active_instance: &mut dyn PhysicalComponent,
) -> (bool, Vec<(interaction::PhysicalPortId, interaction::PhysicalPortId)>) {
    let mut changed = false;

    let mut input_ports_to_remove = Vec::new();
    for (port_id, _port) in &active_instance.physical_ports().physical_input_mapping {
        let is_call = logical_actor
            .image
            .spec
            .input_ports
            .get(&port_id)
            .is_some_and(|p| p.method == edgeless_api::function_instance::PortMethod::Call);

        let is_side_effect = sinks(logical_actor).contains(port_id);

        let port_outputs = logical_actor
            .image
            .spec
            .inner_structure
            .get(&edgeless_api::function_instance::MappingNode::Port(port_id.clone()))
            .map(|destinations| destinations.clone())
            .unwrap_or_default();

        let triggers_output = port_outputs.iter().any(|mapping| {
            if let edgeless_api::function_instance::MappingNode::Port(output_port_id) = mapping {
                if active_instance.physical_ports().physical_output_mapping.contains_key(output_port_id) {
                    return true;
                }
            }

            false
        });

        if !(is_call || is_side_effect || triggers_output) {
            input_ports_to_remove.push(port_id.clone())
        }
    }

    if input_ports_to_remove.len() > 0 {
        changed = true;
    }

    let peer_source_port_entries_to_remove = remove_inputs_and_calculate_removable_peer_sources(&input_ports_to_remove, active_instance);

    (changed, peer_source_port_entries_to_remove)
}

fn remove_unused_output_ports(
    logical_actor: &crate::ir::actor::LogicalActor,
    active_instance: &mut dyn PhysicalComponent,
) -> (bool, Vec<(interaction::PhysicalPortId, interaction::PhysicalPortId)>) {
    let mut changed = false;

    let mut output_ports_to_remove = Vec::new();
    for (port_id, _port) in &active_instance.physical_ports().physical_output_mapping {
        let is_triggered = logical_actor.image.spec.inner_structure.iter().any(|(source, dests)| {
            if !dests.contains(&edgeless_api::function_instance::MappingNode::Port(port_id.clone())) {
                return false;
            }

            match source {
                edgeless_api::function_instance::MappingNode::Port(port_id) => {
                    active_instance.physical_ports().physical_input_mapping.contains_key(port_id)
                }
                edgeless_api::function_instance::MappingNode::SideEffect => true,
            }
        });

        if !is_triggered {
            output_ports_to_remove.push(port_id.clone());
        }
    }

    if output_ports_to_remove.len() > 0 {
        changed = true;
    }

    let peer_destination_port_entries_to_remove = remove_outputs_and_calculate_removable_peer_destinations(&output_ports_to_remove, active_instance);

    (changed, peer_destination_port_entries_to_remove)
}

fn remove_unused_instance(
    logical_actor: &crate::ir::actor::LogicalActor,
    active_instance: &mut dyn PhysicalComponent,
) -> (
    bool,
    Vec<(interaction::PhysicalPortId, interaction::PhysicalPortId)>,
    Vec<(interaction::PhysicalPortId, interaction::PhysicalPortId)>,
) {
    let self_trigger = logical_actor.image.spec.inner_structure.iter().any(|(source, dest)| {
        if let edgeless_api::function_instance::MappingNode::SideEffect = source {
            if dest
                .iter()
                .find(|i| **i == edgeless_api::function_instance::MappingNode::SideEffect)
                .is_some()
            {
                return true;
            }
        }
        false
    });

    let required_output_missing = logical_actor.image.spec.output_ports.iter().any(|(port_id, port)| {
        if !port.optional {
            if !active_instance.physical_ports().physical_output_mapping.contains_key(port_id) {
                return true;
            }
        }
        false
    });

    let required_input_missing = logical_actor.image.spec.input_ports.iter().any(|(port_id, port)| {
        if !port.optional {
            if !active_instance.physical_ports().physical_input_mapping.contains_key(port_id) {
                return true;
            }
        }
        false
    });

    if !required_input_missing
        && !required_output_missing
        && (!active_instance.physical_ports().physical_input_mapping.is_empty()
            || !active_instance.physical_ports().physical_output_mapping.is_empty()
            || self_trigger)
    {
        return (false, vec![], vec![]);
    }

    tracing::debug!(
        "Remove: {} on {}; Inputs Missing: {required_input_missing} Outputs Missing: {required_output_missing}",
        logical_actor.image.spec.behavior_id.id,
        active_instance.id().node_id
    );

    let input_port_ids: Vec<_> = active_instance.physical_ports().physical_input_mapping.keys().cloned().collect();
    let peer_source_port_entries_to_remove = remove_inputs_and_calculate_removable_peer_sources(&input_port_ids, active_instance);
    let output_port_ids: Vec<_> = active_instance.physical_ports().physical_output_mapping.keys().cloned().collect();
    let peer_destination_port_entries_to_remove = remove_outputs_and_calculate_removable_peer_destinations(&output_port_ids, active_instance);

    (true, peer_source_port_entries_to_remove, peer_destination_port_entries_to_remove)
}

fn sinks(logical_actor: &crate::ir::actor::LogicalActor) -> std::collections::BTreeSet<edgeless_api::function_instance::PortId> {
    logical_actor
        .image
        .spec
        .inner_structure
        .iter()
        .filter_map(|(source, dest)| {
            if let edgeless_api::function_instance::MappingNode::Port(port_id) = source {
                if dest.iter().any(|i| *i == edgeless_api::function_instance::MappingNode::SideEffect) {
                    return Some(port_id.clone());
                }
            }
            None
        })
        .collect()
}

fn remove_inputs_and_calculate_removable_peer_sources(
    input_ports_to_remove: &[edgeless_api::function_instance::PortId],
    active_instance: &mut dyn PhysicalComponent,
) -> Vec<(interaction::PhysicalPortId, interaction::PhysicalPortId)> {
    let mut peer_source_port_entries_to_remove = Vec::new();
    for input_to_remove in input_ports_to_remove {
        let Some(removed_entry) = active_instance.physical_ports_mut().physical_input_mapping.remove(&input_to_remove) else {
            tracing::warn!("Port to remove not found.");
            continue;
        };

        let Ok(concrete_port_mapping) =
            crate::ir::interaction::dialect::physical_overlay::PhysicalOverlayDestinationPort::as_concrete(removed_entry.mapping.as_ref())
        else {
            tracing::warn!("Unexpected Port type.");
            continue;
        };

        for source in &concrete_port_mapping.sources {
            peer_source_port_entries_to_remove.push((
                source.clone(),
                interaction::PhysicalPortId {
                    instance: active_instance.id().clone(),
                    port: input_to_remove.clone(),
                },
            ));
        }
    }
    peer_source_port_entries_to_remove
}

fn remove_outputs_and_calculate_removable_peer_destinations(
    output_ports_to_remove: &[edgeless_api::function_instance::PortId],
    active_instance: &mut dyn PhysicalComponent,
) -> Vec<(interaction::PhysicalPortId, interaction::PhysicalPortId)> {
    let mut peer_destination_port_entries_to_remove = Vec::new();
    for output_to_remove in output_ports_to_remove {
        let Some(removed_entry) = active_instance.physical_ports_mut().physical_output_mapping.remove(&output_to_remove) else {
            tracing::warn!("Output to remove not found.");
            continue;
        };

        let Ok(concrete_port_mapping) =
            crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort::as_concrete(removed_entry.mapping.as_ref())
        else {
            tracing::warn!("Bad mapping.");
            continue;
        };

        match &concrete_port_mapping.destination {
            interaction::dialect::physical_overlay::DestinationMapping::Unicast(destination_port) => {
                peer_destination_port_entries_to_remove.push((
                    destination_port.clone(),
                    interaction::PhysicalPortId {
                        instance: active_instance.id(),
                        port: output_to_remove.clone(),
                    },
                ));
            }
            interaction::dialect::physical_overlay::DestinationMapping::Anycast(destination_ports)
            | interaction::dialect::physical_overlay::DestinationMapping::Multicast(destination_ports) => {
                for destination_port in destination_ports {
                    peer_destination_port_entries_to_remove.push((
                        destination_port.clone(),
                        interaction::PhysicalPortId {
                            instance: active_instance.id(),
                            port: output_to_remove.clone(),
                        },
                    ));
                }
            }
        }
    }
    peer_destination_port_entries_to_remove
}

fn remove_peer_source_ports(
    cloned_instances: &mut std::collections::HashMap<uuid::Uuid, ClonedComponentInstance>,
    to_remove: &[(interaction::PhysicalPortId, interaction::PhysicalPortId)],
) -> bool {
    let mut change = false;
    for (source, dest) in to_remove {
        let Some(source_instance) = cloned_instances.get_mut(&source.instance.function_id) else {
            tracing::debug!("Tried to remove port entry from non-existing source peer.");
            continue;
        };

        let Some(active_instance) = source_instance.instance.try_unpack_active_mut() else {
            tracing::debug!("Component instance is not in an active state.");
            continue;
        };

        let port_mapping_entry = active_instance.physical_ports_mut().physical_output_mapping.entry(source.port.clone());

        let std::collections::hash_map::Entry::Occupied(mut port_mapping) = port_mapping_entry else {
            tracing::debug!("Tried to remove destination from non-existing source port.");
            continue;
        };

        let m = port_mapping.get_mut().mapping.as_mut() as &mut dyn std::any::Any;
        let concrete_source_port = m.downcast_mut::<crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort>();

        let Some(concrete_source_port) = concrete_source_port else {
            tracing::debug!("Unexpected Port Type");
            continue;
        };

        match &mut concrete_source_port.destination {
            interaction::dialect::physical_overlay::DestinationMapping::Unicast(_physical_port_id) => {
                port_mapping.remove();
                source_instance.changed = true;
                change = true;
            }
            interaction::dialect::physical_overlay::DestinationMapping::Anycast(physical_port_ids)
            | interaction::dialect::physical_overlay::DestinationMapping::Multicast(physical_port_ids) => {
                let removed = physical_port_ids.remove(&dest);

                if physical_port_ids.is_empty() {
                    port_mapping.remove();
                }

                if removed {
                    source_instance.changed = true;
                    change = true;
                }
            }
        }
    }
    change
}

fn remove_peer_destination_ports(
    cloned_instances: &mut std::collections::HashMap<uuid::Uuid, ClonedComponentInstance>,
    to_remove: &[(interaction::PhysicalPortId, interaction::PhysicalPortId)],
) -> bool {
    let mut change = false;
    for (dest, source) in to_remove {
        let Some(dest_instance) = cloned_instances.get_mut(&dest.instance.function_id) else {
            tracing::debug!("Tried to remove port entry from non-existing destination peer.");
            continue;
        };

        let Some(active_instance) = dest_instance.instance.try_unpack_active_mut() else {
            tracing::debug!("Component instance is not in an active state.");
            continue;
        };

        let std::collections::hash_map::Entry::Occupied(mut destination_port_mapping_entry) =
            active_instance.physical_ports_mut().physical_input_mapping.entry(dest.port.clone())
        else {
            tracing::debug!("Tried to remove source from non-existing destination port.");
            continue;
        };

        let m = destination_port_mapping_entry.get_mut().mapping.as_mut() as &mut dyn std::any::Any;
        let Some(concrete_destination_port) = m.downcast_mut::<crate::ir::interaction::dialect::physical_overlay::PhysicalOverlayDestinationPort>()
        else {
            tracing::debug!("Unexpected Port Type");
            continue;
        };

        let removed = concrete_destination_port.sources.remove(source);

        if concrete_destination_port.sources.is_empty() {
            destination_port_mapping_entry.remove();
        }

        if removed {
            change = true;
            dest_instance.changed = true;
        }
    }
    change
}

#[cfg(test)]
mod test {
    use crate::ir::transformations::StatelessPhysicalTransformation;

    #[test]
    fn remove_if_final_output_missing() {
        // source(instance) -- processor(instance) -- sink(no_instance)

        let source_id = "source".to_string();
        let processor_id = "processor".to_string();
        let sink_id = "sink".to_string();

        let (sink_id, sink_actor) = super::super::dead_component_removal::test::sink_actor(
            sink_id,
            processor_id.clone(),
            super::super::dead_component_removal::test::processor_output(),
        );

        let (processor_id, processor_actor) = super::super::dead_component_removal::test::processor_actor(
            processor_id.clone(),
            source_id.clone(),
            super::super::dead_component_removal::test::source_port_1(),
            Some((sink_id.clone(), super::super::dead_component_removal::test::sink_input())),
        );

        let (source_id, source_actor) = super::super::dead_component_removal::test::source_actor(
            source_id.clone(),
            processor_id.clone(),
            super::super::dead_component_removal::test::processor_input(),
            None,
        );

        let cluster_id = uuid::Uuid::new_v4();

        let node_id = uuid::Uuid::new_v4();
        let source_instance_id = uuid::Uuid::new_v4();

        let (_, processor_instance_id, processor_instance) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&processor_id, &processor_actor)
                .with_state(crate::ir::actor::mock_actor::DesiredPhysicalComponentState::Planned)
                .with_desired_mapping(crate::ir::PhysicalPorts {
                    physical_output_mapping: Default::default(),
                    physical_input_mapping: std::collections::HashMap::from([
                        crate::ir::interaction::dialect::physical_overlay::mock_ports::mock_destination_port(
                            cluster_id.clone(),
                            &super::super::dead_component_removal::test::processor_input(),
                            &[crate::ir::interaction::PhysicalPortId {
                                instance: edgeless_api::function_instance::InstanceId {
                                    node_id: node_id.clone(),
                                    function_id: source_instance_id.clone(),
                                },
                                port: super::super::dead_component_removal::test::source_port_1(),
                            }],
                        ),
                    ]),
                })
                .build();

        let (_, source_instance_id, source_instance) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&source_id, &source_actor)
                .with_state(crate::ir::actor::mock_actor::DesiredPhysicalComponentState::Planned)
                .with_node_id(node_id)
                .with_component_id(source_instance_id)
                .with_desired_mapping(crate::ir::PhysicalPorts {
                    physical_output_mapping: std::collections::HashMap::from([
                        crate::ir::interaction::dialect::physical_overlay::mock_ports::mock_source_port(
                            cluster_id.clone(),
                            &super::super::dead_component_removal::test::source_port_1(),
                            crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(crate::ir::interaction::PhysicalPortId {
                                instance: processor_instance.id().unwrap(),
                                port: super::super::dead_component_removal::test::processor_input(),
                            }),
                        ),
                    ]),
                    physical_input_mapping: Default::default(),
                })
                .build();

        let wf = super::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&source_id, &source_actor, &[(source_instance_id, source_instance)])
            .with_component(&processor_id, &processor_actor, &[(processor_instance_id, processor_instance)])
            .with_component(&sink_id, &sink_actor, &[])
            .build();

        let mut transformation = super::DeadInstanceRemoval::new();

        let changes = transformation.apply(&wf, &crate::ir::Nodes::default(), &crate::ir::Clusters::default());

        assert_eq!(changes.len(), 2);

        let change_map: std::collections::HashMap<_, _> = changes
            .iter()
            .map(|change| {
                let crate::ir::transformations::PhysicalChange::Component(component_change) = &change else {
                    panic!("Unexpected Change");
                };
                (component_change.component_id.clone(), component_change.action.clone())
            })
            .collect();

        assert!(std::matches!(
            change_map.get(&processor_instance_id).unwrap(),
            crate::ir::transformations::PhysicalComponentChangeAction::Delete
        ));

        assert!(std::matches!(
            change_map.get(&processor_instance_id).unwrap(),
            crate::ir::transformations::PhysicalComponentChangeAction::Delete
        ));
    }

    #[test]
    fn dont_change_full_chain() {
        // source(instance) -- processor(instance) -- sink(instance)

        let source_id = "source".to_string();
        let processor_id = "processor".to_string();
        let sink_id = "sink".to_string();

        let (sink_id, sink_actor) = super::super::dead_component_removal::test::sink_actor(
            sink_id,
            processor_id.clone(),
            super::super::dead_component_removal::test::processor_output(),
        );

        let (processor_id, processor_actor) = super::super::dead_component_removal::test::processor_actor(
            processor_id.clone(),
            source_id.clone(),
            super::super::dead_component_removal::test::source_port_1(),
            Some((sink_id.clone(), super::super::dead_component_removal::test::sink_input())),
        );

        let (source_id, source_actor) = super::super::dead_component_removal::test::source_actor(
            source_id.clone(),
            processor_id.clone(),
            super::super::dead_component_removal::test::processor_input(),
            None,
        );

        let cluster_id = uuid::Uuid::new_v4();

        let processor_instance_id = uuid::Uuid::new_v4();
        let node_id = uuid::Uuid::new_v4();
        let source_instance_id = uuid::Uuid::new_v4();

        let (_, sink_instance_id, sink_instance) = crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&sink_id, &sink_actor)
            .with_state(crate::ir::actor::mock_actor::DesiredPhysicalComponentState::Planned)
            .with_desired_mapping(crate::ir::PhysicalPorts {
                physical_output_mapping: Default::default(),
                physical_input_mapping: std::collections::HashMap::from([
                    crate::ir::interaction::dialect::physical_overlay::mock_ports::mock_destination_port(
                        cluster_id.clone(),
                        &super::super::dead_component_removal::test::sink_input(),
                        &[crate::ir::interaction::PhysicalPortId {
                            instance: edgeless_api::function_instance::InstanceId {
                                node_id: node_id.clone(),
                                function_id: processor_instance_id.clone(),
                            },
                            port: super::super::dead_component_removal::test::processor_output(),
                        }],
                    ),
                ]),
            })
            .build();

        let (_, processor_instance_id, processor_instance) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&processor_id, &processor_actor)
                .with_state(crate::ir::actor::mock_actor::DesiredPhysicalComponentState::Planned)
                .with_node_id(node_id.clone())
                .with_component_id(processor_instance_id.clone())
                .with_desired_mapping(crate::ir::PhysicalPorts {
                    physical_output_mapping: std::collections::HashMap::from([
                        crate::ir::interaction::dialect::physical_overlay::mock_ports::mock_source_port(
                            cluster_id.clone(),
                            &super::super::dead_component_removal::test::processor_output(),
                            crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(crate::ir::interaction::PhysicalPortId {
                                instance: sink_instance.id().unwrap(),
                                port: super::super::dead_component_removal::test::sink_input(),
                            }),
                        ),
                    ]),
                    physical_input_mapping: std::collections::HashMap::from([
                        crate::ir::interaction::dialect::physical_overlay::mock_ports::mock_destination_port(
                            cluster_id.clone(),
                            &super::super::dead_component_removal::test::processor_input(),
                            &[crate::ir::interaction::PhysicalPortId {
                                instance: edgeless_api::function_instance::InstanceId {
                                    node_id: node_id.clone(),
                                    function_id: source_instance_id.clone(),
                                },
                                port: super::super::dead_component_removal::test::source_port_1(),
                            }],
                        ),
                    ]),
                })
                .build();

        let (_, source_instance_id, source_instance) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&source_id, &source_actor)
                .with_state(crate::ir::actor::mock_actor::DesiredPhysicalComponentState::Planned)
                .with_node_id(node_id.clone())
                .with_component_id(source_instance_id.clone())
                .with_desired_mapping(crate::ir::PhysicalPorts {
                    physical_output_mapping: std::collections::HashMap::from([
                        crate::ir::interaction::dialect::physical_overlay::mock_ports::mock_source_port(
                            cluster_id.clone(),
                            &super::super::dead_component_removal::test::source_port_1(),
                            crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(crate::ir::interaction::PhysicalPortId {
                                instance: processor_instance.id().unwrap(),
                                port: super::super::dead_component_removal::test::processor_input(),
                            }),
                        ),
                    ]),
                    physical_input_mapping: Default::default(),
                })
                .build();

        let wf = super::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&source_id, &source_actor, &[(source_instance_id, source_instance)])
            .with_component(&processor_id, &processor_actor, &[(processor_instance_id, processor_instance)])
            .with_component(&sink_id, &sink_actor, &[(sink_instance_id, sink_instance)])
            .build();

        let mut transformation = super::DeadInstanceRemoval::new();

        let changes = transformation.apply(&wf, &crate::ir::Nodes::default(), &crate::ir::Clusters::default());

        assert_eq!(changes.len(), 0);
    }
}
