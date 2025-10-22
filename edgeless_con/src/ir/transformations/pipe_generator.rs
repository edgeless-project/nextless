// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use crate::ir::interaction::dialect::InteractionDialect;

use super::super::*;

pub struct PipeGenerator {}

impl PipeGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct PipeGeneratorState {
    pub inner: std::sync::Arc<tokio::sync::Mutex<PipeGeneratorStateInner>>,
}

pub struct PipeGeneratorStateInner {
    pub old: std::collections::HashMap<edgeless_api::link::LinkType, Box<dyn edgeless_api::link::LinkController>>,
    pub multicast_dialect: crate::ir::interaction::dialect::ip_multicast::IpMulticastDialect,
}

impl PipeGeneratorState {
    pub fn new(links: std::collections::HashMap<edgeless_api::link::LinkType, Box<dyn edgeless_api::link::LinkController>>) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(PipeGeneratorStateInner {
                old: links,
                multicast_dialect: crate::ir::interaction::dialect::ip_multicast::IpMulticastDialect::new(),
            })),
        }
    }
}

impl super::StatefulTransformation<PipeGeneratorState> for PipeGenerator {
    fn apply(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        global_state: &PipeGeneratorState,
    ) {
        if workflow.original_request.annotations.contains_key("DISABLE_MULTICAST") {
            return;
        }

        let mcast = edgeless_api::link::LinkType("MULTICAST".to_string());

        let mut srcs = Vec::new();
        let mut dests = Vec::new();

        for (_c_id, c) in workflow.components() {
            let mut current = c.borrow_mut();
            let (_logical_ports, physical_instances) = current.split_view();
            for i in &physical_instances {
                if let Some(i) = i.borrow_mut().try_unpack_materialized_mut() {
                    let cloned_id = i.id().clone();
                    let ports = i.physical_ports();

                    for (out_id, out) in &ports.physical_output_mapping {
                        let out_any = out.mapping.as_ref() as &dyn std::any::Any;
                        let maybe_out = out_any.downcast_ref::<crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort>();

                        let Some(out_mapping) = maybe_out else {
                            continue;
                        };

                        if let crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Multicast(_) = &out_mapping.destination {
                            srcs.push((
                                crate::ir::interaction::PhysicalPortId {
                                    instance: cloned_id.clone(),
                                    port: out_id.clone(),
                                },
                                out_mapping.clone(),
                            ));
                        }
                    }

                    for (input_id, input) in &ports.physical_input_mapping {
                        let input_any = input.mapping.as_ref() as &dyn std::any::Any;
                        let maybe_input =
                            input_any.downcast_ref::<crate::ir::interaction::dialect::physical_overlay::PhysicalOverlayDestinationPort>();

                        let Some(input_mapping) = maybe_input else {
                            continue;
                        };

                        dests.push((
                            crate::ir::interaction::PhysicalPortId {
                                instance: cloned_id,
                                port: input_id.clone(),
                            },
                            input_mapping.clone(),
                        ));
                    }
                }
            }
        }

        let interactions = crate::ir::interaction::dialect::physical_overlay::PhysicalOverlayDialect {}.ports_to_interaction(srcs, dests);

        let mapped_interactions: Vec<_> = interactions
            .into_iter()
            .flat_map(|i| global_state.inner.blocking_lock().multicast_dialect.translate_from_physial_overlay(i))
            .collect();

        let mut replacement_srcs = std::collections::BTreeMap::<
            edgeless_api::function_instance::InstanceId,
            std::collections::BTreeMap<edgeless_api::function_instance::PortId, interaction::dialect::ip_multicast::IpMulticastSourcePort>,
        >::new();
        let mut replacement_dests = std::collections::BTreeMap::<
            edgeless_api::function_instance::InstanceId,
            std::collections::BTreeMap<edgeless_api::function_instance::PortId, interaction::dialect::ip_multicast::IpMulticastDestinationPort>,
        >::new();

        for i in mapped_interactions {
            let mut relevant_nodes = std::collections::BTreeSet::new();

            for sub in &i.subscribers {
                relevant_nodes.insert(sub.instance.node_id.clone());
            }

            for publisher in &i.publishers {
                relevant_nodes.insert(publisher.instance.node_id.clone());
            }

            let relevant_nodes = relevant_nodes.into_iter().map(|n| {
                (
                    n.clone(),
                    nodes.get(&n).unwrap().available_link_types().get(&mcast).unwrap().clone(),
                    global_state
                        .inner
                        .blocking_lock()
                        .multicast_dialect
                        .config_for(i.link_id.clone(), n)
                        .unwrap(),
                    false,
                )
            });

            workflow.links.entry(i.link_id.clone()).or_insert(link::WorkflowLink {
                id: i.link_id.clone(),
                class: mcast.clone(),
                materialized: false,
                nodes: relevant_nodes.collect(),
            });

            let (s, d) = global_state.inner.blocking_lock().multicast_dialect.interaction_to_ports(i);
            for (port_id, source_spec) in s {
                replacement_srcs
                    .entry(port_id.instance.clone())
                    .or_default()
                    .insert(port_id.port.clone(), source_spec);
            }
            for (port_id, dest_spec) in d {
                replacement_dests
                    .entry(port_id.instance.clone())
                    .or_default()
                    .insert(port_id.port.clone(), dest_spec);
            }
        }

        for (_c_id, c) in workflow.components() {
            let mut current = c.borrow_mut();
            let (_logical_ports, physical_instances) = current.split_view();
            for i in &physical_instances {
                if let Some(i) = i.borrow_mut().try_unpack_materialized_mut() {
                    let cloned_id = i.id();
                    let ports = i.physical_ports();

                    let inputs = replacement_dests.remove(&cloned_id).unwrap_or_default();
                    let outputs = replacement_srcs.remove(&cloned_id).unwrap_or_default();

                    for (input_port, port_spec) in inputs {
                        ports.physical_input_mapping.insert(
                            input_port,
                            crate::ir::interaction::DestiantionPortMapping {
                                dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                                    base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                                    constraints: std::collections::BTreeSet::new(),
                                },
                                mapping: Box::new(port_spec),
                            },
                        );
                    }

                    for (output_port, port_spec) in outputs {
                        ports.physical_output_mapping.insert(
                            output_port,
                            crate::ir::interaction::SourcePortMapping {
                                dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                                    base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                                    constraints: std::collections::BTreeSet::new(),
                                },
                                mapping: Box::new(port_spec),
                            },
                        );
                    }
                }
            }
        }
    }
}
