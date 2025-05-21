// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

mod feasibility;
mod scoring;
pub mod strategy;

use super::super::*;
use scoring::ScoreableRuntime;

pub struct DefaultPlacement<P: strategy::PlacementStrategy> {
    placement_strategy: P,
}

impl<P: strategy::PlacementStrategy> DefaultPlacement<P> {
    pub fn new(placement_strategy: P) -> Self {
        Self { placement_strategy }
    }
}

impl<P: strategy::PlacementStrategy> super::StatefulTransformation<P::GlobalState> for DefaultPlacement<P> {
    fn apply(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &mut P::GlobalState,
    ) {
        for (f_id, function) in &mut workflow.functions {
            let function = function.borrow_mut();
            for i in &function.instances {
                let mut i = i.borrow_mut();
                match &*i {
                    PhysicalComponentState::Planned => {
                        log::info!("Planned : {}", function.instances.len());
                        let candidates = find_candidates_for_actor(&function, nodes);
                        let dst = self.placement_strategy.select_candidate(candidates, global_state);

                        if let Some(dst) = dst {
                            *i = PhysicalComponentState::Existing(actor::PhysicalActor {
                                id: edgeless_api::function_instance::InstanceId::new(dst.node_id),
                                runtime_type: dst.runtime.id(),
                                desired_mapping: PhysicalPorts::default(),
                                image: None,
                                materialized: None,
                                creation_tine: std::time::Instant::now(),
                            });
                        } else {
                            log::info!("Found no viable node for {} in {}", &f_id, workflow.id.workflow_id);
                        }
                    }
                    _ => {
                        //NOOP
                    }
                }
            }
        }

        for resource in workflow.resources.values_mut() {
            let resource = resource.borrow_mut();

            for r in &resource.instances {
                let mut r = r.borrow_mut();
                match &*r {
                    PhysicalComponentState::Planned => {
                        let dst = select_node_for_resource(&resource, nodes);
                        if let Some(dst) = dst {
                            *r = PhysicalComponentState::Existing(resource::PhysicalResource {
                                id: edgeless_api::function_instance::InstanceId::new(dst),
                                desired_mapping: PhysicalPorts::default(),
                                materialized: None,
                                creation_time: std::time::Instant::now(),
                            });
                        }
                    }
                    _ => {
                        //NOOP
                    }
                }
            }

            if resource.instances.is_empty() {}
        }

        for subflow in workflow.subflows.values_mut() {
            let subflow = subflow.borrow_mut();

            for s in &subflow.instances {
                let mut s = s.borrow_mut();
                match &*s {
                    PhysicalComponentState::Planned => {
                        let dst = select_cluster_for_subflow(&subflow, peer_clusters);
                        if let Some(dst) = dst {
                            *s = PhysicalComponentState::Existing(subflow::PhysicalSubFlow {
                                id: edgeless_api::function_instance::InstanceId::new(dst),
                                desired_mapping: PhysicalPorts::default(),
                                materialized: None,
                                creation_time: std::time::Instant::now(),
                            });
                        }
                    }
                    _ => {
                        //NOOP
                    }
                }
            }

            if subflow.instances.is_empty() && subflow.instances.is_empty() {}
        }

        {
            let proxy = workflow.proxy.borrow_mut();
            for p in &proxy.instances {
                let mut p = p.borrow_mut();
                match &*p {
                    PhysicalComponentState::Planned => {
                        let dst = select_node_for_proxy(&proxy, nodes);
                        if let Some(dst) = dst {
                            *p = PhysicalComponentState::Existing(proxy::PhyiscalProxy {
                                id: edgeless_api::function_instance::InstanceId::new(dst),
                                desired_mapping: PhysicalPorts::default(),
                                materialized: None,
                                creation_time: std::time::Instant::now(),
                            });
                        }
                    }
                    _ => {
                        //NOOP
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct Candidate<'a> {
    node_id: edgeless_api::function_instance::NodeId,
    runtime: crate::ir::Runtime<'a>,
}

fn find_candidates_for_actor<'b>(actor: &actor::LogicalActor, nodes: &'b crate::ir::Nodes) -> Vec<Candidate<'b>> {
    let mut candiates = Vec::new();

    for node in nodes.values() {
        let mut node_cadidates = feasibility::feasible_node_runtime_candidates(actor, *node);

        node_cadidates.sort_by(|a, b| a.runtime.efficiency_score().total_cmp(&b.runtime.efficiency_score()));
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
