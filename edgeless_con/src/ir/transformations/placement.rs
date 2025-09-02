// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod candidate_filter;
mod feasibility;
mod scoring;
pub mod strategy;

use crate::ir::{support::image_cache, transformations::placement::candidate_filter::FilterStrategy};

use super::super::*;
use scoring::ScoreableRuntime;

pub struct DefaultPlacement<P: strategy::PlacementStrategy> {
    placement_strategy: P,
    dynamic_colocation_filter: candidate_filter::dynamic_colocation::DynamicColocation,
    static_colocation_filter: candidate_filter::static_colocation::StaticColocation,
}

impl<P: strategy::PlacementStrategy> DefaultPlacement<P> {
    pub fn new(placement_strategy: P) -> Self {
        Self {
            placement_strategy,
            dynamic_colocation_filter: candidate_filter::dynamic_colocation::DynamicColocation::new(),
            static_colocation_filter: candidate_filter::static_colocation::StaticColocation::new(),
        }
    }
}

pub struct PlacementState<'a, P: strategy::PlacementStrategy> {
    pub strategy_state: &'a P::GlobalState,
    pub image_chache: &'a crate::ir::support::image_cache::ImageCache,
}

impl<'a, P: strategy::PlacementStrategy> super::StatefulTransformation<PlacementState<'a, P>> for DefaultPlacement<P> {
    fn apply(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &PlacementState<P>,
    ) {
        for (f_id, function) in &workflow.functions {
            let mut function = function.borrow_mut();

            let num_active_instances = function.instances.iter().filter(|i| i.borrow().try_unpack_active().is_some()).count();

            let mut new_instances = Vec::new();
            for i in &function.instances {
                let mut i = i.borrow_mut();
                match &mut *i {
                    PhysicalComponentState::Requested => {
                        let new_instance = self.spawn_new(workflow, f_id.clone(), &function, nodes, global_state, true, num_active_instances < 1);
                        if let Some(new_instance) = new_instance {
                            *i = new_instance;
                        } else {
                            log::info!(
                                "Placement;Requested Instance: Found no viable node for {} in {}",
                                &f_id,
                                workflow.id.workflow_id
                            );
                        }
                    }
                    PhysicalComponentState::MigrationRequested(c) => {
                        let new_instance = self.spawn_new(workflow, f_id.clone(), &function, nodes, global_state, false, false);
                        if let Some(new_instance) = new_instance {
                            let new_id = new_instance.id().unwrap();
                            if new_id.node_id == c.id().node_id {
                                log::info!(
                                    "Placement;MigratingInstance: Node would be equal {}({}). {}",
                                    f_id,
                                    c.id(),
                                    function.instances.len()
                                );
                                i.abort_migration();
                            } else {
                                log::info!(
                                    "Placement;MigratingInstance: Found Replacement node for {} in {} ({}); Will migrate: {} -> {}",
                                    &f_id,
                                    workflow.id.workflow_id,
                                    c.id(),
                                    c.id().node_id,
                                    new_id.node_id
                                );
                                new_instances.push(std::cell::RefCell::new(new_instance));
                                i.mark_migrating_away(new_id);
                            }
                        } else {
                            log::info!("No Instance Found");
                            i.abort_migration();
                        }
                    }
                    PhysicalComponentState::Lost(_) => {
                        let new_instance = self.spawn_new(workflow, f_id.clone(), &function, nodes, global_state, false, num_active_instances < 1);
                        if let Some(new_instance) = new_instance {
                            let new_id = new_instance.id().unwrap();
                            new_instances.push(std::cell::RefCell::new(new_instance));
                            i.mark_lost_replaced(new_id);
                        }
                    }
                    PhysicalComponentState::Dead(_) => {
                        let new_instance = self.spawn_new(workflow, f_id.clone(), &function, nodes, global_state, false, num_active_instances < 1);
                        if let Some(new_instance) = new_instance {
                            let new_id = new_instance.id().unwrap();
                            new_instances.push(std::cell::RefCell::new(new_instance));
                            i.mark_dead_replaced(new_id);
                        }
                    }
                    _ => {
                        //NOOP
                    }
                }
            }
            function.instances.extend(new_instances);
        }

        for (resource_id, resource) in &mut workflow.resources {
            let resource = resource.borrow_mut();

            for r in &resource.instances {
                let mut r = r.borrow_mut();
                match &*r {
                    PhysicalComponentState::Requested => {
                        let dst = select_node_for_resource(&resource, nodes);
                        if let Some(dst) = dst {
                            *r = PhysicalComponentState::Materialized(Box::new(resource::PhysicalResource {
                                id: edgeless_api::function_instance::InstanceId::new(dst),
                                desired_mapping: PhysicalPorts::default(),
                                materialized: None,
                                creation_time: std::time::Instant::now(),
                                class: resource.class.clone(),
                                component_name: resource_id.clone(),
                                configuration: resource.configurations.clone(),
                            }));
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
                    PhysicalComponentState::Requested => {
                        let dst = select_cluster_for_subflow(&subflow, peer_clusters);
                        if let Some(dst) = dst {
                            *s = PhysicalComponentState::Materialized(Box::new(subflow::PhysicalSubFlow {
                                id: edgeless_api::function_instance::InstanceId::new(dst),
                                desired_mapping: PhysicalPorts::default(),
                                materialized: None,
                                creation_time: std::time::Instant::now(),
                                internal_ports: InternalPorts::default(),
                            }));
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
                    PhysicalComponentState::Requested => {
                        let dst = select_node_for_proxy(&proxy, nodes);
                        if let Some(dst) = dst {
                            *p = PhysicalComponentState::Materialized(Box::new(proxy::PhyiscalProxy {
                                id: edgeless_api::function_instance::InstanceId::new(dst),
                                desired_mapping: PhysicalPorts::default(),
                                materialized: None,
                                creation_time: std::time::Instant::now(),
                                external_ports: ExternalPorts::default(),
                            }));
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
    pub(crate) node_id: edgeless_api::function_instance::NodeId,
    pub(crate) runtime: crate::ir::Runtime<'a>,
    pub(crate) runtime_features: std::collections::BTreeSet<crate::ir::DialectFeature>,
}
impl<P: strategy::PlacementStrategy> DefaultPlacement<P> {
    fn spawn_new(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        logical_name: String,
        function: &crate::ir::actor::LogicalActor,
        nodes: &crate::ir::Nodes,
        global_state: &PlacementState<P>,
        new_instance: bool,
        urgent: bool,
    ) -> Option<PhysicalComponentState> {
        let candidates = find_candidates_for_actor(function, nodes, new_instance, urgent, global_state.image_chache);
        let mut filtered = self.dynamic_colocation_filter.filter_candidates(function, candidates, workflow);
        if filtered.len() > 1 {
            filtered = self.static_colocation_filter.filter_candidates(function, filtered, workflow);
        };
        let dst = self.placement_strategy.select_candidate(filtered, global_state.strategy_state);

        if let Some(dst) = dst {
            log::info!("Target: {}, {:?}", dst.runtime.id(), dst.runtime_features);
            let new_id = edgeless_api::function_instance::InstanceId::new(dst.node_id);
            let new_instance = actor::PhysicalActor {
                id: new_id,
                runtime_type: crate::ir::actor::DialectType {
                    base_type: dst.runtime.id(),
                    features: dst.runtime_features,
                },
                desired_mapping: PhysicalPorts::default(),
                //TODO (use correct image)
                image: function.image.main_image.clone(),
                behavior_spec: function.image.spec.clone(),
                materialized: None,
                creation_time: std::time::Instant::now(),
                component_name: logical_name,
                annotations: function.annotations.clone(),
            };

            let mut new_component_state = PhysicalComponentState::request_new_instance();
            new_component_state.plan_creation(Box::new(new_instance));
            Some(new_component_state)
        } else {
            None
        }
    }
}

fn find_candidates_for_actor<'b>(
    actor: &actor::LogicalActor,
    nodes: &'b crate::ir::Nodes,
    new_instance: bool,
    urgent: bool,
    image_cache: &crate::ir::support::image_cache::ImageCache,
) -> Vec<Candidate<'b>> {
    let mut candiates = Vec::new();

    if urgent {
        for node in nodes.values() {
            let mut node_cadidates: Vec<_> = feasibility::feasible_node_runtime_candidates(actor, *node, new_instance)
                .into_iter()
                .filter(|c| {
                    let image_ident = crate::ir::actor::BehaviorImageId {
                        behavior_id: actor.image.spec.behavior_id.clone(),
                        enabled_ports: crate::ir::actor::EnabledPorts {
                            enabled_inputs: actor.enabled_inputs().iter().cloned().collect(),
                            enabled_outputs: actor.enabled_outputs().iter().cloned().collect(),
                        },
                        dialect_type: crate::ir::actor::DialectType {
                            base_type: c.runtime.id(),
                            features: c.runtime_features.clone(),
                        },
                    };

                    for extra in actor.image.extra_images.clone() {
                        log::info!("Got Extra Image");
                        image_cache.insert_blocking(extra);
                    }

                    let existing = image_cache.get_blocking(&image_ident);

                    match existing {
                        image_cache::CacheResult::NotFound => {}
                        image_cache::CacheResult::PartialMatch(partial_match) => {
                            log::info!("Partial Image");
                            let existing_images: Vec<_> = partial_match
                                .same_or_more_ports(&image_ident)
                                .same_runtime_feature_subset(&image_ident)
                                .into();
                            if !existing_images.is_empty() {
                                return true;
                            }
                        }
                        image_cache::CacheResult::FullMatch(_) => {
                            log::info!("Full Image");
                            return true;
                        }
                    }
                    false
                })
                .collect();
            node_cadidates.sort_by(|a, b| a.runtime.efficiency_score().total_cmp(&b.runtime.efficiency_score()));
            if let Some(c) = node_cadidates.pop() {
                candiates.push(c);
            }
        }
    }

    if !candiates.is_empty() {
        return candiates;
    }

    for node in nodes.values() {
        let mut node_cadidates = feasibility::feasible_node_runtime_candidates(actor, *node, new_instance);

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
