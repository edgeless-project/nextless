// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use crate::ir::interaction::{dialect::InteractionDialect, DestiantionPortMapping, SourcePortMapping};

use super::super::*;

pub struct LogicalInteractionNormalizer {}

impl LogicalInteractionNormalizer {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatelessTransformation for LogicalInteractionNormalizer {
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, _nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        let mut srcs = Vec::new();
        let mut dests = Vec::new();

        // Find Targets
        for (cid, component) in &mut workflow.components() {
            component
                .borrow_mut()
                .logical_ports_mut()
                .logical_input_mapping
                .retain(|port_id, port_mapping| {
                    let m = port_mapping.mapping.as_ref() as &dyn std::any::Any;
                    let p = m.downcast_ref::<crate::ir::interaction::dialect::topic_pub_sub::TopicPubSubDestinationPort>();

                    match p {
                        Some(topic_port) => {
                            dests.push((
                                interaction::LogicalPortId {
                                    component: cid.to_string(),
                                    port: port_id.clone(),
                                },
                                topic_port.clone(),
                            ));
                            false
                        }
                        None => true,
                    }
                });
            component
                .borrow_mut()
                .logical_ports_mut()
                .logical_output_mapping
                .retain(|port_id, port_mapping| {
                    let m = port_mapping.mapping.as_ref() as &dyn std::any::Any;
                    let p = m.downcast_ref::<crate::ir::interaction::dialect::topic_pub_sub::TopicPubSubSourcePort>();

                    match p {
                        Some(topic_port) => {
                            srcs.push((
                                interaction::LogicalPortId {
                                    component: cid.to_string(),
                                    port: port_id.clone(),
                                },
                                topic_port.clone(),
                            ));
                            false
                        }
                        None => true,
                    }
                })
        }

        let interactions = crate::ir::interaction::dialect::topic_pub_sub::TopicPubSubDialect {}.ports_to_interaction(srcs, dests);

        let mapped_interactions: Vec<interaction::dialect::logical_overlay::LogicalOverlayInteraction> = interactions
            .into_iter()
            .flat_map(|i| crate::ir::interaction::dialect::topic_pub_sub::TopicPubSubDialect::translate_to_logical_overlay(i))
            .collect();

        let mut replacement_srcs = std::collections::BTreeMap::<
            String,
            std::collections::BTreeMap<edgeless_api::function_instance::PortId, interaction::dialect::logical_overlay::LogicalOverlaySourcePort>,
        >::new();
        let mut replacement_dests = std::collections::BTreeMap::<
            String,
            std::collections::BTreeMap<edgeless_api::function_instance::PortId, interaction::dialect::logical_overlay::LogicalOverlayDestinationPort>,
        >::new();

        for i in mapped_interactions {
            let (s, d) = crate::ir::interaction::dialect::logical_overlay::LogicalOverlayDialect {}.interaction_to_ports(i);
            for (port_id, source_spec) in s {
                replacement_srcs
                    .entry(port_id.component.clone())
                    .or_default()
                    .insert(port_id.port.clone(), source_spec);
            }
            for (port_id, dest_spec) in d {
                replacement_dests
                    .entry(port_id.component.clone())
                    .or_default()
                    .insert(port_id.port.clone(), dest_spec);
            }
        }

        for (cid, component) in &mut workflow.components() {
            let mut c = component.borrow_mut();
            let ports = c.logical_ports_mut();

            let inputs = replacement_dests.remove(&cid.to_string()).unwrap_or_default();
            let outputs = replacement_srcs.remove(&cid.to_string()).unwrap_or_default();

            for (input_port_id, input_port_spec) in inputs {
                ports.logical_input_mapping.insert(
                    input_port_id,
                    DestiantionPortMapping {
                        dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                            base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                            constraints: std::collections::BTreeSet::new(),
                        },
                        mapping: Box::new(input_port_spec),
                    },
                );
            }

            for (ouput_port_id, output_port_spec) in outputs {
                ports.logical_output_mapping.insert(
                    ouput_port_id,
                    SourcePortMapping {
                        dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                            base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                            constraints: std::collections::BTreeSet::new(),
                        },
                        mapping: Box::new(output_port_spec),
                    },
                );
            }
        }
    }
}
