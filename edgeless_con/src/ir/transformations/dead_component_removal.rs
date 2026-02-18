// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub use super::super::*;

pub struct DeadComponentRemoval {}

impl super::StatelessLogicalTransformation for DeadComponentRemoval {
    #[tracing::instrument(name = "dead_component_removal", skip_all)]
    fn apply(&mut self, workflow: &crate::ir::workflow::ActiveWorkflow) -> Vec<super::LogicalChange> {
        if workflow.feature_flags.disable_application_optimization {
            return vec![];
        }

        let mut cloned_components: std::collections::HashMap<String, (crate::ir::logical_model::LogicalComponent, bool)> = workflow
            .components_with_instances()
            .map(|(id, component, _)| (id.to_string(), (component.clone(), false)))
            .collect();

        let mut total_change = false;
        let mut iteration_change = true;
        while iteration_change {
            iteration_change = false;
            iteration_change = Self::remove_unused_inputs(&mut cloned_components) || iteration_change;
            iteration_change = Self::remove_unused_outputs(&mut cloned_components) || iteration_change;
            iteration_change = Self::remove_unused_functions(&mut cloned_components) || iteration_change;
            if iteration_change {
                total_change = true;
            }
        }

        if !total_change {
            return vec![];
        }

        let mut required_changes = Vec::new();

        if cloned_components.len() != workflow.components_with_instances().count() {
            for (id, _, _) in workflow.components_with_instances() {
                if !cloned_components.contains_key(id) {
                    required_changes.push(crate::ir::transformations::LogicalChange::Component(
                        crate::ir::transformations::LogicalComponentChange {
                            component_id: id.to_string(),
                            action: transformations::LogicalComponentChangeAction::Delete,
                        },
                    ));
                }
            }
        }

        for (id, (component, changed)) in cloned_components {
            if changed {
                required_changes.push(crate::ir::transformations::LogicalChange::Component(
                    crate::ir::transformations::LogicalComponentChange {
                        component_id: id.to_string(),
                        action: transformations::LogicalComponentChangeAction::Update(component),
                    },
                ));
            }
        }

        return required_changes;
    }
}

impl DeadComponentRemoval {
    pub fn new() -> Self {
        Self {}
    }

    fn remove_unused_outputs(slf: &mut std::collections::HashMap<String, (crate::ir::logical_model::LogicalComponent, bool)>) -> bool {
        let mut changed = false;

        let mut input_links_to_remove = Vec::new();

        for (f_id, (f, component_changed)) in slf.iter_mut() {
            let crate::ir::logical_model::LogicalComponent::Actor(actor) = f else {
                continue;
            };

            let inner: std::collections::BTreeMap<edgeless_api::function_instance::MappingNode, Vec<edgeless_api::function_instance::MappingNode>> =
                actor.image.spec.inner_structure.clone();
            let ports = &mut f.logical_ports_mut();
            ports.logical_output_mapping.retain(|output_id, output_spec: &mut LogicalOutput| {
                // assert!(!std::matches!(output_spec, super::super::LogicalOutput::Topic(_)));

                let m = output_spec.mapping.as_ref() as &dyn std::any::Any;
                let p = m.downcast_ref::<crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort>();

                let Some(mapping) = p else {
                    panic!("Bad Mapping");
                };

                let this = edgeless_api::function_instance::MappingNode::Port(output_id.clone());
                for (src, dests) in &inner {
                    if dests.contains(&this) {
                        match src {
                            edgeless_api::function_instance::MappingNode::Port(port) => {
                                if ports.logical_input_mapping.contains_key(port) {
                                    return true;
                                }
                            }
                            edgeless_api::function_instance::MappingNode::SideEffect => {
                                return true;
                            }
                        }
                    }
                }

                tracing::debug!("Optimizer wants to remove output: {}", output_id.0);
                let mut to_remove = match &mapping.destination {
                    interaction::dialect::logical_overlay::DestinationMapping::Unicast(logical_port_id) => {
                        vec![(
                            (logical_port_id.component.clone(), logical_port_id.port.clone()),
                            (f_id.clone(), output_id.clone()),
                        )]
                    }
                    interaction::dialect::logical_overlay::DestinationMapping::Anycast(logical_port_ids) => logical_port_ids
                        .iter()
                        .map(|logical_port_id| {
                            (
                                (logical_port_id.component.clone(), logical_port_id.port.clone()),
                                (f_id.clone(), output_id.clone()),
                            )
                        })
                        .collect(),
                    interaction::dialect::logical_overlay::DestinationMapping::Multicast(logical_port_ids) => logical_port_ids
                        .iter()
                        .map(|logical_port_id| {
                            (
                                (logical_port_id.component.clone(), logical_port_id.port.clone()),
                                (f_id.clone(), output_id.clone()),
                            )
                        })
                        .collect(),
                };
                input_links_to_remove.append(&mut to_remove);
                changed = true;
                *component_changed = true;
                false
            });
        }

        for ((target_component_id, target_port_id), (source_component_id, source_port_id)) in &input_links_to_remove {
            if let Some((source, component_changed)) = slf.get_mut(target_component_id) {
                let mut remove = false;

                if let Some(input_spec) = source.logical_ports_mut().logical_input_mapping.get_mut(target_port_id) {
                    let m = input_spec.mapping.as_mut() as &mut dyn std::any::Any;
                    let p = m.downcast_mut::<crate::ir::interaction::dialect::logical_overlay::LogicalOverlayDestinationPort>();
                    if let Some(input) = p {
                        let before = input.sources.len();
                        input
                            .sources
                            .retain(|source_port| &source_port.component != source_component_id && &source_port.port != source_port_id);

                        if input.sources.len() != before {
                            *component_changed = true;
                        }

                        if input.sources.is_empty() {
                            *component_changed = true;
                            remove = true;
                        }
                    }
                }
                if remove {
                    source.logical_ports_mut().logical_input_mapping.remove(target_port_id);
                }
            }
        }

        changed
    }

    fn remove_unused_inputs(slf: &mut std::collections::HashMap<String, (crate::ir::logical_model::LogicalComponent, bool)>) -> bool {
        let mut changed = false;

        let mut output_links_to_remove = Vec::new();

        for (f_id, (f, component_changed)) in slf.iter_mut() {
            let crate::ir::logical_model::LogicalComponent::Actor(actor) = f else {
                continue;
            };

            let behavior = actor.image.clone();
            let f_ports = &mut f.logical_ports_mut();
            f_ports.logical_input_mapping.retain(|input_id, input_spec| {
                let m = input_spec.mapping.as_mut() as &mut dyn std::any::Any;
                let p = m.downcast_mut::<crate::ir::interaction::dialect::logical_overlay::LogicalOverlayDestinationPort>();

                if let Some(input_spec) = p {
                    let port_method = behavior.spec.input_ports.get(input_id).unwrap().method.clone();
                    // We only need to worry about removing casts as calls will always be usefull
                    if port_method == edgeless_api::function_instance::PortMethod::Cast {
                        let inner_for_this = behavior
                            .spec
                            .inner_structure
                            .get(&edgeless_api::function_instance::MappingNode::Port(input_id.clone()));
                        if let Some(inner_targets) = inner_for_this {
                            if inner_targets.contains(&edgeless_api::function_instance::MappingNode::SideEffect) {
                                return true;
                            } else {
                                for output in f_ports.logical_output_mapping.keys() {
                                    if inner_targets.contains(&edgeless_api::function_instance::MappingNode::Port(output.clone())) {
                                        return true;
                                    }
                                }
                            }
                        }

                        tracing::debug!("Optimizer wants to remove input: {}", input_id.0);

                        output_links_to_remove.append(
                            &mut input_spec
                                .sources
                                .iter()
                                .map(|o| ((o.component.clone(), o.port.clone()), (f_id.clone(), input_id.clone())))
                                .collect(),
                        );
                        changed = true;
                        *component_changed = true;
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            });
        }

        for ((source_id, source_port_id), (dest_id, dest_port_id)) in &output_links_to_remove {
            if let Some((source, component_changed)) = slf.get_mut(source_id) {
                let mut remove = false;
                if let Some(source_port) = source.logical_ports_mut().logical_output_mapping.get_mut(source_port_id) {
                    let m = source_port.mapping.as_mut() as &mut dyn std::any::Any;
                    let p = m.downcast_mut::<crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort>();

                    let Some(logical_source_port) = p else {
                        continue;
                    };

                    match &mut logical_source_port.destination {
                        interaction::dialect::logical_overlay::DestinationMapping::Unicast(logical_port_id) => {
                            if &logical_port_id.component == dest_id && &logical_port_id.port == dest_port_id {
                                remove = true;
                                *component_changed = true;
                            }
                        }
                        interaction::dialect::logical_overlay::DestinationMapping::Anycast(logical_port_ids) => {
                            let before = logical_port_ids.len();
                            logical_port_ids
                                .retain(|logical_port_id| !(&logical_port_id.component == dest_id && &logical_port_id.port == dest_port_id));

                            if logical_port_ids.len() != before {
                                *component_changed = true;
                            }
                            if logical_port_ids.is_empty() {
                                *component_changed = true;
                                remove = true;
                            }
                        }
                        interaction::dialect::logical_overlay::DestinationMapping::Multicast(logical_port_ids) => {
                            let before = logical_port_ids.len();
                            logical_port_ids
                                .retain(|logical_port_id| !(&logical_port_id.component == dest_id && &logical_port_id.port == dest_port_id));
                            if logical_port_ids.len() != before {
                                *component_changed = true;
                            }
                            if logical_port_ids.is_empty() {
                                *component_changed = true;
                                remove = true;
                            }
                        }
                    }
                }
                if remove {
                    source.logical_ports_mut().logical_output_mapping.remove(source_port_id);
                }
            }
        }

        changed
    }

    fn remove_unused_functions(slf: &mut std::collections::HashMap<String, (crate::ir::logical_model::LogicalComponent, bool)>) -> bool {
        let before = slf.len();

        slf.retain(|f_id, (f_spec, _)| {
            let crate::ir::logical_model::LogicalComponent::Actor(actor) = &f_spec else {
                return true;
            };

            let triggered_by_side_effect: std::collections::BTreeSet<edgeless_api::function_instance::MappingNode> = actor
                .image
                .spec
                .inner_structure
                .get(&edgeless_api::function_instance::MappingNode::SideEffect)
                .unwrap_or(&vec![])
                .iter()
                .cloned()
                .collect();

            if triggered_by_side_effect.contains(&edgeless_api::function_instance::MappingNode::SideEffect) {
                return true;
            }

            if f_spec.logical_ports().logical_input_mapping.is_empty() {
                let mut keep_if_triggered_by_side_effect = false;

                for (port_id, _port) in &f_spec.logical_ports().logical_output_mapping {
                    if triggered_by_side_effect.contains(&edgeless_api::function_instance::MappingNode::Port(port_id.clone())) {
                        keep_if_triggered_by_side_effect = true;
                        break;
                    }
                }

                if !keep_if_triggered_by_side_effect {
                    return false;
                }
            }

            if f_spec.logical_ports().logical_output_mapping.is_empty() {
                let mut keep_if_triggers_side_effect = false;

                for (port_id, _port) in &f_spec.logical_ports().logical_input_mapping {
                    if let Some(port_mapping) = actor
                        .image
                        .spec
                        .inner_structure
                        .get(&edgeless_api::function_instance::MappingNode::Port(port_id.clone()))
                    {
                        if port_mapping.contains(&edgeless_api::function_instance::MappingNode::SideEffect) {
                            keep_if_triggers_side_effect = true;
                            break;
                        }
                    }
                }

                if !keep_if_triggers_side_effect {
                    return false;
                }
            }

            return true;
        });
        before != slf.len()
    }
}

#[cfg(test)]
mod test {
    use crate::ir::{interaction::dialect::AsConcreteSourcePort, transformations::StatelessLogicalTransformation};

    #[test]
    fn remove_unused_chain() {
        //        - sink
        // source
        //        - processor - (unused output)
        //
        let source_id = "source".to_string();
        let source_port_1 = edgeless_api::function_instance::PortId("source_1".to_string());
        let source_port_2 = edgeless_api::function_instance::PortId("source_2".to_string());
        let mut source_image = crate::ir::test::mock_actor_image();
        source_image.spec.output_ports.insert(
            source_port_1.clone(),
            edgeless_api::function_instance::Port {
                id: source_port_1.clone(),
                method: edgeless_api::function_instance::PortMethod::Cast,
                data_type: edgeless_api::function_instance::PortDataType("test".to_string()),
                return_data_type: None,
            },
        );
        source_image.spec.output_ports.insert(
            source_port_2.clone(),
            edgeless_api::function_instance::Port {
                id: source_port_2.clone(),
                method: edgeless_api::function_instance::PortMethod::Cast,
                data_type: edgeless_api::function_instance::PortDataType("test".to_string()),
                return_data_type: None,
            },
        );
        source_image.spec.inner_structure.insert(
            edgeless_api::function_instance::MappingNode::SideEffect,
            vec![
                edgeless_api::function_instance::MappingNode::Port(source_port_1.clone()),
                edgeless_api::function_instance::MappingNode::Port(source_port_2.clone()),
            ],
        );

        let sink_id = "sink".to_string();
        let sink_input = edgeless_api::function_instance::PortId("sink_input".to_string());
        let mut sink_image = crate::ir::test::mock_actor_image();
        sink_image.spec.inner_structure.insert(
            edgeless_api::function_instance::MappingNode::Port(sink_input.clone()),
            vec![edgeless_api::function_instance::MappingNode::SideEffect],
        );
        sink_image.spec.input_ports.insert(
            sink_input.clone(),
            edgeless_api::function_instance::Port {
                id: sink_input.clone(),
                method: edgeless_api::function_instance::PortMethod::Cast,
                data_type: edgeless_api::function_instance::PortDataType("test".to_string()),
                return_data_type: None,
            },
        );

        let processor_id = "processor".to_string();
        let processor_input = edgeless_api::function_instance::PortId("processor_input".to_string());
        let processor_output = edgeless_api::function_instance::PortId("processor_output".to_string());
        let mut processor_image = crate::ir::test::mock_actor_image();
        processor_image.spec.inner_structure.insert(
            edgeless_api::function_instance::MappingNode::Port(processor_input.clone()),
            vec![edgeless_api::function_instance::MappingNode::Port(processor_output.clone())],
        );
        processor_image.spec.input_ports.insert(
            processor_input.clone(),
            edgeless_api::function_instance::Port {
                id: processor_input.clone(),
                method: edgeless_api::function_instance::PortMethod::Cast,
                data_type: edgeless_api::function_instance::PortDataType("test".to_string()),
                return_data_type: None,
            },
        );
        processor_image.spec.output_ports.insert(
            processor_output.clone(),
            edgeless_api::function_instance::Port {
                id: processor_output.clone(),
                method: edgeless_api::function_instance::PortMethod::Cast,
                data_type: edgeless_api::function_instance::PortDataType("test".to_string()),
                return_data_type: None,
            },
        );

        let (source_id, source_actor) = crate::ir::actor::mock_actor::MockActorBuilder::default()
            .with_logical_id(source_id.clone())
            .with_image(source_image)
            .with_logical_ports(crate::ir::LogicalPorts {
                logical_output_mapping: std::collections::HashMap::from([
                    super::interaction::dialect::logical_overlay::mock_ports::mock_source_port(
                        &source_port_1,
                        crate::ir::interaction::dialect::logical_overlay::DestinationMapping::Unicast(crate::ir::interaction::LogicalPortId {
                            component: sink_id.clone(),
                            port: sink_input.clone(),
                        }),
                    ),
                    super::interaction::dialect::logical_overlay::mock_ports::mock_source_port(
                        &source_port_2,
                        crate::ir::interaction::dialect::logical_overlay::DestinationMapping::Unicast(crate::ir::interaction::LogicalPortId {
                            component: processor_id.clone(),
                            port: processor_input.clone(),
                        }),
                    ),
                ]),
                logical_input_mapping: Default::default(),
            })
            .build();

        let (sink_id, sink_actor) = crate::ir::actor::mock_actor::MockActorBuilder::default()
            .with_logical_id(sink_id.clone())
            .with_image(sink_image)
            .with_logical_ports(crate::ir::LogicalPorts {
                logical_output_mapping: Default::default(),
                logical_input_mapping: std::collections::HashMap::from([
                    super::interaction::dialect::logical_overlay::mock_ports::mock_destination_port(
                        &sink_input,
                        &[crate::ir::interaction::LogicalPortId {
                            component: source_id.clone(),
                            port: source_port_1.clone(),
                        }],
                    ),
                ]),
            })
            .build();

        let (processor_id, processor_actor) = crate::ir::actor::mock_actor::MockActorBuilder::default()
            .with_logical_id(processor_id.clone())
            .with_image(processor_image)
            .with_logical_ports(crate::ir::LogicalPorts {
                logical_output_mapping: Default::default(),
                logical_input_mapping: std::collections::HashMap::from([
                    super::interaction::dialect::logical_overlay::mock_ports::mock_destination_port(
                        &processor_input,
                        &[crate::ir::interaction::LogicalPortId {
                            component: source_id.clone(),
                            port: source_port_2.clone(),
                        }],
                    ),
                ]),
            })
            .build();

        let wf = super::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&source_id, &source_actor, &[])
            .with_component(&sink_id, &sink_actor, &[])
            .with_component(&processor_id, &processor_actor, &[])
            .build();

        let mut transformation = super::DeadComponentRemoval::new();

        let changes = transformation.apply(&wf);

        assert_eq!(changes.len(), 2);

        let mut change_map = std::collections::HashMap::<String, crate::ir::transformations::LogicalComponentChangeAction>::new();

        for change in changes {
            let crate::ir::transformations::LogicalChange::Component(logical_component_change) = change;

            change_map.insert(logical_component_change.component_id.clone(), logical_component_change.action);
        }
        assert_eq!(change_map.len(), 2);

        assert!(std::matches!(
            change_map.get(&processor_id).unwrap(),
            crate::ir::transformations::LogicalComponentChangeAction::Delete
        ));

        if let crate::ir::transformations::LogicalComponentChangeAction::Update(updated_component) = change_map.get(&source_id).unwrap() {
            assert_eq!(updated_component.logical_ports().logical_input_mapping.len(), 0);
            assert_eq!(updated_component.logical_ports().logical_output_mapping.len(), 1);

            let mapping = crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort::as_concrete(
                updated_component
                    .logical_ports()
                    .logical_output_mapping
                    .get(&source_port_1)
                    .unwrap()
                    .mapping
                    .as_ref(),
            )
            .unwrap();

            let crate::ir::interaction::dialect::logical_overlay::DestinationMapping::Unicast(unicast_target) = &mapping.destination else {
                panic!("Unexpected Mapping");
            };

            assert_eq!(unicast_target.component, sink_id);
            assert_eq!(unicast_target.port, sink_input);
        } else {
            panic!("Unexpected Change");
        }
    }
}
