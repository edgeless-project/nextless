// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub use super::super::*;

pub struct DeadComponentRemoval {}

impl super::Transformation for DeadComponentRemoval {
    fn apply(&mut self, workflow: &mut workflow::ActiveWorkflow) {
        let mut changed = true;
        while changed {
            changed = false;
            changed = Self::remove_unused_inputs(workflow) || changed;
            changed = Self::remove_unused_outputs(workflow) || changed;
        }
    }
}

impl DeadComponentRemoval {
    pub fn new() -> Self {
        Self {}
    }

    fn remove_unused_outputs(slf: &mut workflow::ActiveWorkflow) -> bool {
        let mut changed = false;

        let mut input_links_to_remove = Vec::new();

        for (f_id, f) in &mut slf.functions {
            let mut f = f.borrow_mut();
            let inner: std::collections::HashMap<
                edgeless_api::function_instance::MappingNode,
                std::collections::HashSet<edgeless_api::function_instance::MappingNode>,
            > = f.image.class.inner_structure.clone();
            let ports = &mut f.logical_ports();
            ports.logical_output_mapping.retain(|output_id, output_spec: &mut LogicalOutput| {
                assert!(!std::matches!(output_spec, super::super::LogicalOutput::Topic(_)));
                let this = edgeless_api::function_instance::MappingNode::Port(output_id.clone());
                for (src, dests) in &inner {
                    if dests.contains(&this) {
                        match src {
                            edgeless_api::function_instance::MappingNode::Port(port) => {
                                if ports.logical_input_mapping.contains_key(&port) {
                                    return true;
                                }
                                log::info!("Not an Active Input");
                            }
                            edgeless_api::function_instance::MappingNode::SideEffect => {
                                return true;
                            }
                        }
                    }
                }

                let mut to_remove = match output_spec {
                    LogicalOutput::DirectTarget(target_node_id, target_port_id) => {
                        vec![((target_node_id.clone(), target_port_id.clone()), (f_id.clone(), output_id.clone()))]
                    }
                    LogicalOutput::AnyOfTargets(targets) => targets
                        .iter()
                        .map(|(target_node_id, target_port_id)| ((target_node_id.clone(), target_port_id.clone()), (f_id.clone(), output_id.clone())))
                        .collect(),
                    LogicalOutput::AllOfTargets(targets) => targets
                        .iter()
                        .map(|(target_node_id, target_port_id)| ((target_node_id.clone(), target_port_id.clone()), (f_id.clone(), output_id.clone())))
                        .collect(),
                    LogicalOutput::Topic(_) => vec![],
                };
                input_links_to_remove.append(&mut to_remove);
                changed = true;
                false
            });
        }

        for ((target_component_id, target_port_id), (source_component_id, source_port_id)) in &input_links_to_remove {
            if let Some(source) = slf.functions.get_mut(target_component_id) {
                let mut source = source.borrow_mut();
                let mut remove = false;
                if let Some(source_port) = source.logical_ports().logical_input_mapping.get_mut(target_port_id) {
                    if let LogicalInput::Direct(sources) = source_port {
                        sources.retain(|(s_id, s_p_id)| s_id != source_component_id && s_p_id != source_port_id);
                        if sources.len() == 0 {
                            remove = true;
                        }
                    }
                }
                if remove {
                    source.logical_ports().logical_input_mapping.remove(target_port_id);
                }
            }
        }

        changed
    }

    fn remove_unused_inputs(slf: &mut workflow::ActiveWorkflow) -> bool {
        let mut changed = false;

        let mut output_links_to_remove = Vec::new();

        for (f_id, f) in &mut slf.functions {
            let mut f = f.borrow_mut();
            let class = f.image.class.clone();
            let f_ports = &mut f.logical_ports();
            f_ports.logical_input_mapping.retain(|input_id, input_spec| {
                if let LogicalInput::Direct(mapped_inputs) = input_spec {
                    let port_method = class.inputs.get(input_id).unwrap().method.clone();
                    // We only need to worry about removing casts as calls will always be usefull
                    if port_method == edgeless_api::function_instance::PortMethod::Cast {
                        let inner_for_this = class
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

                        output_links_to_remove.append(
                            &mut mapped_inputs
                                .iter()
                                .map(|(o_comp, o_port)| ((o_comp.clone(), o_port.clone()), (f_id.clone(), input_id.clone())))
                                .collect(),
                        );
                        changed = true;
                        return false;
                    } else {
                        return true;
                    }
                } else {
                    return true;
                }
            });
        }

        for ((source_id, source_port_id), (dest_id, dest_port_id)) in &output_links_to_remove {
            if let Some(source) = slf.functions.get_mut(source_id) {
                let mut source = source.borrow_mut();
                let mut remove = false;
                if let Some(source_port) = source.logical_ports().logical_output_mapping.get_mut(source_port_id) {
                    match source_port {
                        LogicalOutput::DirectTarget(target_id, target_port_id) => {
                            if target_id == dest_id && target_port_id == dest_port_id {
                                remove = true;
                            }
                        }
                        LogicalOutput::AnyOfTargets(targets) => {
                            targets.retain(|(target_id, target_port_id)| !(target_id == dest_id && target_port_id == dest_port_id));
                            if targets.len() == 0 {
                                remove = true;
                            }
                        }
                        LogicalOutput::AllOfTargets(targets) => {
                            targets.retain(|(target_id, target_port_id)| !(target_id == dest_id && target_port_id == dest_port_id));
                            if targets.len() == 0 {
                                remove = true;
                            }
                        }
                        LogicalOutput::Topic(_) => {}
                    }
                }
                if remove {
                    source.logical_ports().logical_output_mapping.remove(source_port_id);
                }
            }
        }

        changed
    }
}
