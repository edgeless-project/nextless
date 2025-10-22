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

impl super::StatelessTransformation for PhysicalConnectionMapper {
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, _nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        let components = workflow
            .components()
            .into_iter()
            .map(|(id, spec)| {
                (
                    id.to_string(),
                    spec.borrow_mut()
                        .instances()
                        .iter()
                        .filter_map(|i| i.borrow_mut().try_unpack_active().map(|i| i.id()))
                        .collect(),
                )
            })
            .collect::<std::collections::HashMap<String, Vec<edgeless_api::function_instance::InstanceId>>>();

        for component_id in components.keys() {
            let mut component = workflow.get_component(component_id).unwrap().borrow_mut();
            let (logical_ports, physical_instances) = component.split_view();

            for (output_id, output) in &logical_ports.logical_output_mapping {
                let m = output.mapping.as_ref() as &dyn std::any::Any;
                let p = m.downcast_ref::<crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort>();

                let Some(logical_output) = p else {
                    continue;
                };

                match &logical_output.destination {
                    interaction::dialect::logical_overlay::DestinationMapping::Unicast(logical_port_id) => {
                        let target_component = &logical_port_id.component;
                        let target_port_id = &logical_port_id.port;

                        let mut instances = components.get(target_component).unwrap().clone();

                        if instances.len() > 1 {
                            log::info!("Temporarily breaking single target assumption!");
                            for c_instance in &physical_instances {
                                if let Some(c_instance) = c_instance.borrow_mut().try_unpack_active_mut() {
                                    c_instance.physical_ports().physical_output_mapping.insert(
                                        output_id.clone(),
                                        crate::ir::interaction::SourcePortMapping {
                                            dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                                                base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                                                constraints: std::collections::BTreeSet::new(),
                                            },
                                            mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                                                destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Anycast(
                                                    instances
                                                        .iter()
                                                        .map(|i| crate::ir::interaction::PhysicalPortId {
                                                            instance: *i,
                                                            port: target_port_id.clone(),
                                                        })
                                                        .collect(),
                                                ),
                                            }),
                                        },
                                    );
                                }
                            }
                        } else if let Some(id) = instances.pop() {
                            for c_instance in &physical_instances {
                                if let Some(c_instance) = c_instance.borrow_mut().try_unpack_active_mut() {
                                    c_instance.physical_ports().physical_output_mapping.insert(
                                        output_id.clone(),
                                        crate::ir::interaction::SourcePortMapping {
                                            dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                                                base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                                                constraints: std::collections::BTreeSet::new(),
                                            },
                                            mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                                                destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(
                                                    crate::ir::interaction::PhysicalPortId {
                                                        instance: id,
                                                        port: target_port_id.clone(),
                                                    },
                                                ),
                                            }),
                                        },
                                    );
                                }
                            }
                        }
                    }
                    interaction::dialect::logical_overlay::DestinationMapping::Anycast(logical_port_ids) => {
                        let mut instances = Vec::new();
                        for logical_port_id in logical_port_ids {
                            let target_id = &logical_port_id.component;
                            let port_id = &logical_port_id.port;
                            instances.append(
                                &mut components
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
                        for c_instance in &physical_instances {
                            if let Some(c_instance) = c_instance.borrow_mut().try_unpack_materialized_mut() {
                                c_instance.physical_ports().physical_output_mapping.insert(
                                    output_id.clone(),
                                    crate::ir::interaction::SourcePortMapping {
                                        dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                                            base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                                            constraints: std::collections::BTreeSet::new(),
                                        },
                                        mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                                            destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Anycast(
                                                instances.clone(),
                                            ),
                                        }),
                                    },
                                );
                            }
                        }
                    }
                    interaction::dialect::logical_overlay::DestinationMapping::Multicast(logical_port_ids) => {
                        let mut instances = Vec::new();
                        for logical_port_id in logical_port_ids {
                            let target_id = &logical_port_id.component;
                            let port_id = &logical_port_id.port;
                            instances.append(
                                &mut components
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
                        for c_instance in &physical_instances {
                            if let Some(c_instance) = c_instance.borrow_mut().try_unpack_materialized_mut() {
                                c_instance.physical_ports().physical_output_mapping.insert(
                                    output_id.clone(),
                                    crate::ir::interaction::SourcePortMapping {
                                        dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                                            base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                                            constraints: std::collections::BTreeSet::new(),
                                        },
                                        mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                                            destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Multicast(
                                                instances.clone(),
                                            ),
                                        }),
                                    },
                                );
                            }
                        }
                    }
                }
            }

            for (input_id, input) in &logical_ports.logical_input_mapping {
                let any_mapping = input.mapping.as_ref() as &dyn std::any::Any;
                let maybe_mapping = any_mapping.downcast_ref::<crate::ir::interaction::dialect::logical_overlay::LogicalOverlayDestinationPort>();

                let Some(mapping) = maybe_mapping else {
                    continue;
                };

                let mut sources = Vec::new();
                for logical_port_id in &mapping.sources {
                    let target_id = &logical_port_id.component;
                    let port_id = &logical_port_id.port;
                    sources.append(
                        &mut components
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
                for c_instance in &physical_instances {
                    if let Some(c_instance) = c_instance.borrow_mut().try_unpack_materialized_mut() {
                        c_instance.physical_ports().physical_input_mapping.insert(
                            input_id.clone(),
                            crate::ir::interaction::DestiantionPortMapping {
                                dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                                    base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                                    constraints: std::collections::BTreeSet::new(),
                                },
                                mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlayDestinationPort {
                                    sources: sources.clone(),
                                }),
                            },
                        );
                    }
                }
            }
        }
    }
}
