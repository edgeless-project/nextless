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

        let mut cloned_components: std::collections::HashMap<String, crate::ir::logical_model::LogicalComponent> = workflow
            .components_with_instances()
            .map(|(id, component, _)| (id.to_string(), component.clone()))
            .collect();

        let mut changed = true;
        while changed {
            changed = false;
            changed = Self::remove_unused_inputs(&mut cloned_components) || changed;
            changed = Self::remove_unused_outputs(&mut cloned_components) || changed;
            if changed {
                Self::remove_unused_functions(&mut cloned_components);
            }
        }

        return vec![];
    }
}

impl DeadComponentRemoval {
    pub fn new() -> Self {
        Self {}
    }

    fn remove_unused_outputs(slf: &mut std::collections::HashMap<String, crate::ir::logical_model::LogicalComponent>) -> bool {
        let mut changed = false;

        let mut input_links_to_remove = Vec::new();

        for (f_id, f) in slf.iter_mut() {
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
                false
            });
        }

        for ((target_component_id, target_port_id), (source_component_id, source_port_id)) in &input_links_to_remove {
            if let Some(source) = slf.get_mut(target_component_id) {
                let mut remove = false;

                if let Some(input_spec) = source.logical_ports_mut().logical_input_mapping.get_mut(target_port_id) {
                    let m = input_spec.mapping.as_mut() as &mut dyn std::any::Any;
                    let p = m.downcast_mut::<crate::ir::interaction::dialect::logical_overlay::LogicalOverlayDestinationPort>();
                    if let Some(input) = p {
                        input
                            .sources
                            .retain(|source_port| &source_port.component != source_component_id && &source_port.port != source_port_id);
                        if input.sources.is_empty() {
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

    fn remove_unused_inputs(slf: &mut std::collections::HashMap<String, crate::ir::logical_model::LogicalComponent>) -> bool {
        let mut changed = false;

        let mut output_links_to_remove = Vec::new();

        for (f_id, f) in slf.iter_mut() {
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
            if let Some(source) = slf.get_mut(source_id) {
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
                            }
                        }
                        interaction::dialect::logical_overlay::DestinationMapping::Anycast(logical_port_ids) => {
                            logical_port_ids
                                .retain(|logical_port_id| !(&logical_port_id.component == dest_id && &logical_port_id.port == dest_port_id));
                            if logical_port_ids.is_empty() {
                                remove = true;
                            }
                        }
                        interaction::dialect::logical_overlay::DestinationMapping::Multicast(logical_port_ids) => {
                            logical_port_ids
                                .retain(|logical_port_id| !(&logical_port_id.component == dest_id && &logical_port_id.port == dest_port_id));
                            if logical_port_ids.is_empty() {
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

    fn remove_unused_functions(slf: &mut std::collections::HashMap<String, crate::ir::logical_model::LogicalComponent>) {
        slf.retain(|_f_id, f_spec| {
            !f_spec.logical_ports().logical_input_mapping.is_empty() || !f_spec.logical_ports().logical_output_mapping.is_empty()
        });
    }
}
