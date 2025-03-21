// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

mod feasibility;
mod scoring;
mod strategy;

use scoring::ScoreableRuntime;

use super::super::*;

pub struct DefaultPlacement {
    placement_strategy: Box<dyn strategy::PlacementStrategy>,
}

impl DefaultPlacement {
    pub fn new(placement_strategy: &str) -> Self {
        Self {
            placement_strategy: match placement_strategy {
                "random" => Box::new(strategy::random::Random::new()),
                "weighted_random" => Box::new(strategy::weighted_random::WeightedRandom::new()),
                _ => {
                    panic!("Bad Placement Strategy");
                }
            },
        }
    }
}

impl super::Transformation for DefaultPlacement {
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, nodes: &crate::ir::Nodes, peer_clusters: &crate::ir::Clusters) {
        for (f_id, function) in &mut workflow.functions {
            let mut function = function.borrow_mut();
            if function.instances.is_empty() {
                let candidates = find_candidates_for_actor(&function, nodes);
                let dst = self.placement_strategy.select_candidate(candidates);

                // let dst = self
                //     .orchestration_logic
                //     .blocking_lock()
                //     .next(nodes, &function.image.format, &function.annotations);

                if let Some(dst) = dst {
                    function.instances.push(std::cell::RefCell::new(actor::PhysicalActor {
                        id: edgeless_api::function_instance::InstanceId::new(dst.node_id),
                        desired_mapping: PhysicalPorts::default(),
                        image: None,
                        materialized: None,
                    }))
                } else {
                    log::info!("Found no viable node for {} in {}", &f_id, workflow.id.workflow_id);
                }
            } else {
                // This is a test that shows the use of the Materialized Representation
                for i in &function.instances() {
                    let instance = i.borrow_mut();
                    if let Some(materialized) = &instance.materialized_state() {
                        let mut materialized = materialized.borrow_mut();
                        if let Some(v) = materialized
                            .runtime_statistics()
                            .and_then(|s| s.invocation_rate_abs(std::time::Duration::from_secs(120)))
                        {
                            log::debug!("Invocation Rate: {}", v)
                        }
                        if let Some(v) = materialized
                            .runtime_statistics()
                            .and_then(|s| Some(s.invocations_rate_abs_by_port(std::time::Duration::from_secs(120))))
                        {
                            log::debug!("Invocation Rate By Port: {:?}", v);
                        }

                        if let Some(v) = materialized
                            .runtime_statistics()
                            .and_then(|s| s.duration_mean_secs(std::time::Duration::from_secs(120)))
                        {
                            log::debug!("Mean Duration MS: {}", v * 1000.0);
                        }
                        if let Some(v) = materialized
                            .runtime_statistics()
                            .and_then(|s| s.duration_soft_limit_rate_rel(std::time::Duration::from_secs(120)))
                        {
                            log::debug!("Duration Soft Limit Score: {}", v);
                        }

                        if let Some(v) = materialized
                            .runtime_statistics()
                            .and_then(|s| s.error_rate_rel(std::time::Duration::from_secs(120)))
                        {
                            log::debug!("Error Rate {}", v);
                        }

                        for (p_id, p) in &materialized.materialized_ports().materialized_inputs {
                            if let Some(v) = p
                                .runtime_statistics()
                                .and_then(|s| s.message_rate_abs(std::time::Duration::from_secs(120)))
                            {
                                log::debug!("Port Rate {}: {}", p_id.0, v);
                            }
                            if let Some(v) = p
                                .runtime_statistics()
                                .and_then(|s| Some(s.message_rate_abs_by_peer(std::time::Duration::from_secs(120))))
                            {
                                log::debug!("Port By Peer Rate {}: {:?}", p_id.0, v);
                            }
                            if let Some(v) = p
                                .runtime_statistics()
                                .and_then(|s| s.message_size_mean_bytes(std::time::Duration::from_secs(120)))
                            {
                                log::debug!("Port Size {}: {}", p_id.0, v);
                            }
                            if let Some(v) = p
                                .runtime_statistics()
                                .and_then(|s| Some(s.message_size_mean_byte_by_peer(std::time::Duration::from_secs(120))))
                            {
                                log::debug!("Port By Peer Size {}: {:?}", p_id.0, v);
                            }
                        }
                    }
                }
            }
        }

        for (_, resource) in &mut workflow.resources {
            let mut resource = resource.borrow_mut();
            if resource.instances.is_empty() {
                let dst = select_node_for_resource(&resource, nodes);
                if let Some(dst) = dst {
                    resource.instances.push(std::cell::RefCell::new(resource::PhysicalResource {
                        id: edgeless_api::function_instance::InstanceId::new(dst),
                        desired_mapping: PhysicalPorts::default(),
                        materialized: None,
                    }));
                }
            }
        }

        for (_, subflow) in &mut workflow.subflows {
            let mut subflow = subflow.borrow_mut();
            if subflow.instances.is_empty() && subflow.instances.is_empty() {
                let dst = select_cluster_for_subflow(&subflow, peer_clusters);
                if let Some(dst) = dst {
                    subflow.instances.push(std::cell::RefCell::new(subflow::PhysicalSubFlow {
                        id: edgeless_api::function_instance::InstanceId::new(dst),
                        desired_mapping: PhysicalPorts::default(),
                        materialized: None,
                    }))
                }
            }
        }

        {
            let mut proxy = workflow.proxy.borrow_mut();
            if (!proxy.logical_ports.logical_input_mapping.is_empty() || !proxy.logical_ports.logical_output_mapping.is_empty())
                && proxy.instances.is_empty()
            {
                let dst = select_node_for_proxy(&proxy, nodes);
                if let Some(dst) = dst {
                    proxy.instances.push(std::cell::RefCell::new(proxy::PhyiscalProxy {
                        id: edgeless_api::function_instance::InstanceId::new(dst),
                        desired_mapping: PhysicalPorts::default(),
                        materialized: None,
                    }));
                }
            }
        }
    }
}

#[derive(Clone)]
struct Candidate<'a> {
    node_id: edgeless_api::function_instance::NodeId,
    runtime: crate::ir::Runtime<'a>,
}

fn find_candidates_for_actor<'a, 'b>(actor: &'a actor::LogicalActor, nodes: &'b crate::ir::Nodes) -> Vec<Candidate<'b>> {
    let mut candiates = Vec::new();

    for (_node_id, node) in nodes {
        let mut node_cadidates = feasibility::feasible_node_runtime_candidates(actor, *node);

        node_cadidates.sort_by(|a, b| b.runtime.efficiency_score().total_cmp(&a.runtime.efficiency_score()));
        if let Some(c) = node_cadidates.pop() {
            candiates.push(c);
        }
    }

    candiates
}

fn select_node_for_resource(resource: &resource::LogicalResource, nodes: &crate::ir::Nodes) -> Option<edgeless_api::function_instance::NodeId> {
    if let Some((id, _)) = nodes
        .iter()
        .find(|(_, n)| n.available_resource_providers().iter().any(|(_, r)| r.class_type() == resource.class))
    {
        Some(*id)
    } else {
        None
    }
}

fn select_node_for_proxy(_proxy: &proxy::LogicalProxy, nodes: &crate::ir::Nodes) -> Option<edgeless_api::function_instance::NodeId> {
    for (node_id, node) in nodes {
        if node.is_proxy() {
            return Some(*node_id);
        }
    }
    None
}

fn select_cluster_for_subflow(
    _subflow: &subflow::LogicalSubFlow,
    _clusters: &crate::ir::Clusters,
) -> Option<edgeless_api::function_instance::NodeId> {
    // for (cluster_id, cluster) in clusters {
    //     // TODO Proper Selection
    //     return Some(*cluster_id);
    // }
    None
}
