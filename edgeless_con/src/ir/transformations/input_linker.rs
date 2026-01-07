// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::super::*;

pub struct InputLinker {}

impl InputLinker {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatelessTransformation for InputLinker {
    #[tracing::instrument(name = "input_linker", skip_all)]
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, _nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        let mut inputs = std::collections::HashMap::<
            String,
            std::collections::HashMap<edgeless_api::function_instance::PortId, Vec<interaction::LogicalPortId>>,
        >::new();

        for (out_cid, fdesc) in workflow.components() {
            for (out_port, mapping) in &fdesc.borrow_mut().logical_ports().logical_output_mapping {
                let m = mapping.mapping.as_ref() as &dyn std::any::Any;
                let p = m.downcast_ref::<crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort>();

                let Some(mapping) = p else {
                    continue;
                };

                match &mapping.destination {
                    interaction::dialect::logical_overlay::DestinationMapping::Unicast(logical_port_id) => inputs
                        .entry(logical_port_id.component.clone())
                        .or_default()
                        .entry(logical_port_id.port.clone())
                        .or_default()
                        .push(interaction::LogicalPortId {
                            component: out_cid.to_string(),
                            port: out_port.clone(),
                        }),
                    interaction::dialect::logical_overlay::DestinationMapping::Anycast(logical_port_ids) => {
                        for logical_port_id in logical_port_ids {
                            inputs
                                .entry(logical_port_id.component.clone())
                                .or_default()
                                .entry(logical_port_id.port.clone())
                                .or_default()
                                .push(interaction::LogicalPortId {
                                    component: out_cid.to_string(),
                                    port: out_port.clone(),
                                })
                        }
                    }
                    interaction::dialect::logical_overlay::DestinationMapping::Multicast(logical_port_ids) => {
                        for logical_port_id in logical_port_ids {
                            inputs
                                .entry(logical_port_id.component.clone())
                                .or_default()
                                .entry(logical_port_id.port.clone())
                                .or_default()
                                .push(interaction::LogicalPortId {
                                    component: out_cid.to_string(),
                                    port: out_port.clone(),
                                })
                        }
                    }
                }
            }
        }

        for (targed_fid, links) in inputs {
            if let Some(target) = workflow.functions.get_mut(&targed_fid) {
                for (target_port, sources) in links {
                    target.borrow_mut().logical_ports.logical_input_mapping.insert(
                        target_port.clone(),
                        interaction::DestiantionPortMapping {
                            dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                                base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                                constraints: std::collections::BTreeSet::new(),
                            },
                            mapping: Box::new(interaction::dialect::logical_overlay::LogicalOverlayDestinationPort { sources: sources }),
                        },
                    );
                }
            } else if let Some(target) = workflow.resources.get_mut(&targed_fid) {
                // Some(&mut target.borrow_mut().ports)
                for (target_port, sources) in links {
                    target.borrow_mut().logical_ports.logical_input_mapping.insert(
                        target_port.clone(),
                        interaction::DestiantionPortMapping {
                            dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                                base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                                constraints: std::collections::BTreeSet::new(),
                            },
                            mapping: Box::new(interaction::dialect::logical_overlay::LogicalOverlayDestinationPort { sources: sources }),
                        },
                    );
                }
            }
        }
    }
}
