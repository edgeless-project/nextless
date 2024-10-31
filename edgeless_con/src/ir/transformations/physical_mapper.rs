// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::super::*;
pub struct PhysicalConnectionMapper {}

impl PhysicalConnectionMapper {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::Transformation for PhysicalConnectionMapper {
    fn apply(&mut self, workflow: &mut workflow::ActiveWorkflow) {
        let components = workflow
            .components()
            .into_iter()
            .map(|(id, spec)| (id.to_string(), spec.borrow_mut().instance_ids()))
            .collect::<std::collections::HashMap<String, Vec<edgeless_api::function_instance::InstanceId>>>();

        for (component_id, _) in &components {
            let mut component = workflow.get_component(component_id).unwrap().borrow_mut();
            let (logical_ports, physical_instances) = component.split_view();

            for (output_id, output) in &logical_ports.logical_output_mapping {
                match output {
                    LogicalOutput::DirectTarget(target_component, target_port_id) => {
                        let mut instances = components.get(target_component).unwrap().clone();
                        if let Some(id) = instances.pop() {
                            for c_instance in &physical_instances {
                                c_instance
                                    .borrow_mut()
                                    .physical_ports()
                                    .physical_output_mapping
                                    .insert(output_id.clone(), PhysicalOutput::Single(id, target_port_id.clone()));
                            }
                        }
                    }
                    LogicalOutput::AnyOfTargets(targets) => {
                        let mut instances = Vec::new();
                        for (target_id, port_id) in targets {
                            instances.append(
                                &mut components
                                    .get(target_id)
                                    .unwrap()
                                    .iter()
                                    .map(|target| (*target, port_id.clone()))
                                    .collect(),
                            )
                        }
                        for c_instance in &physical_instances {
                            c_instance
                                .borrow_mut()
                                .physical_ports()
                                .physical_output_mapping
                                .insert(output_id.clone(), PhysicalOutput::Any(instances.clone()));
                        }
                    }
                    LogicalOutput::AllOfTargets(targets) => {
                        let mut instances = Vec::new();
                        for (target_id, port_id) in targets {
                            instances.append(
                                &mut components
                                    .get(target_id)
                                    .unwrap()
                                    .iter()
                                    .map(|target| (*target, port_id.clone()))
                                    .collect(),
                            )
                        }
                        for c_instance in &physical_instances {
                            c_instance
                                .borrow_mut()
                                .physical_ports()
                                .physical_output_mapping
                                .insert(output_id.clone(), PhysicalOutput::All(instances.clone()));
                        }
                    }
                    LogicalOutput::Topic(_) => {}
                }
            }
        }
    }
}
