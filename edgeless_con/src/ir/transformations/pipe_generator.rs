// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::super::*;

pub struct PipeGenerator {
    existing_links: std::collections::HashMap<(String, edgeless_api::function_instance::PortId), edgeless_api::link::LinkInstanceId>,
}

impl PipeGenerator {
    pub fn new() -> Self {
        Self {
            existing_links: std::collections::HashMap::new(),
        }
    }
}

pub struct PipeGeneratorState {
    pub inner:
        std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<edgeless_api::link::LinkType, Box<dyn edgeless_api::link::LinkController>>>>,
}

impl PipeGeneratorState {
    pub fn new(links: std::collections::HashMap<edgeless_api::link::LinkType, Box<dyn edgeless_api::link::LinkController>>) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(links)),
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
        if workflow.original_request.annotations.get("DISABLE_MULTICAST").is_some() {
            return;
        }

        let mcast = edgeless_api::link::LinkType("MULTICAST".to_string());

        let mut new_links = Vec::<(edgeless_api::link::LinkInstanceId, link::WorkflowLink)>::new();

        for (c_id, c) in workflow.components() {
            let mut current = c.borrow_mut();
            let (logical_ports, physical_instances) = current.split_view();
            for i in &physical_instances {
                if let Some(i) = i.borrow_mut().try_unpack_mut() {
                    let own_id = i.id().node_id;
                    for (out_id, out) in &mut i.physical_ports().physical_output_mapping {
                        if let edgeless_api::common::Output::All(targets) = out {
                            if targets.len() >= 2 {
                                let new_link = if let Some(existing) = self.existing_links.get(&(c_id.to_string(), out_id.clone())) {
                                    existing.clone()
                                } else {
                                    let mut target_nodes: std::collections::HashSet<_> = targets.iter().map(|(t_id, _)| t_id.node_id).collect();
                                    target_nodes.insert(own_id);
                                    let new_link = global_state
                                        .inner
                                        .blocking_lock()
                                        .get_mut(&mcast)
                                        .unwrap()
                                        .new_link(target_nodes.clone().into_iter().collect())
                                        .unwrap();

                                    let node_links: Vec<_> = target_nodes
                                        .iter()
                                        .map(|n| {
                                            (
                                                *n,
                                                nodes.get(n).unwrap().available_link_types().get(&mcast).unwrap().clone(),
                                                global_state
                                                    .inner
                                                    .blocking_lock()
                                                    .get(&mcast)
                                                    .unwrap()
                                                    .config_for(new_link.clone(), *n)
                                                    .unwrap(),
                                                false,
                                            )
                                        })
                                        .collect();

                                    new_links.push((
                                        new_link.clone(),
                                        link::WorkflowLink {
                                            id: new_link.clone(),
                                            class: mcast.clone(),
                                            materialized: false,
                                            nodes: node_links,
                                        },
                                    ));

                                    self.existing_links.insert((c_id.to_string(), out_id.clone()), new_link.clone());

                                    new_link
                                };
                                *out = PhysicalOutput::Link(new_link.clone());

                                let logical_port = logical_ports.logical_output_mapping.get(out_id).unwrap();
                                if let edgeless_api::workflow_instance::PortMapping::AllOfTargets(logical_targets) = logical_port {
                                    for (target_name, target_port_id) in logical_targets {
                                        workflow
                                            .get_component(target_name)
                                            .unwrap()
                                            .borrow_mut()
                                            .instances()
                                            .iter()
                                            .for_each(|i| {
                                                if let Some(i) = i.borrow_mut().try_unpack_mut() {
                                                    i.physical_ports()
                                                        .physical_input_mapping
                                                        .insert(target_port_id.clone(), PhysicalInput::Link(new_link.clone()));
                                                }
                                            });
                                    }
                                } else {
                                    panic!("Mapping is Wrong!");
                                }
                            }
                        }
                    }
                }
            }
        }

        for (id, spec) in new_links {
            workflow.links.insert(id, spec);
        }
    }
}
