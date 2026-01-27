// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod candidate_filter;
mod feasibility;
mod scoring;
pub mod strategy;

use std::str::FromStr;

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

struct PlacementConstraints {
    node_filters: crate::ir::component::NodeFilters,
    /// Used to indicate that the system should prefer fast deployment over efficiency.
    urgent: bool,
}

#[derive(Clone)]
pub struct Candidate<'a> {
    pub(crate) node_id: edgeless_api::function_instance::NodeId,
    pub(crate) dest_image: actor::ImageState,
    pub(crate) runtime: crate::ir::Runtime<'a>,
}

impl<'a, P: strategy::PlacementStrategy> super::StatefulTransformation<PlacementState<'a, P>> for DefaultPlacement<P> {
    #[tracing::instrument(name = "placement", skip_all)]
    fn apply(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &PlacementState<P>,
    ) {
        for (f_id, function) in &workflow.functions {
            let mut function = function.borrow_mut();
            self.process_actor(workflow, f_id, &mut *function, nodes, global_state);
        }

        for (resource_id, resource) in &mut workflow.resources {
            let mut resource = resource.borrow_mut();

            let r_class_clone = resource.class.to_string();
            let r_configuration_clone = resource.configurations.clone();

            resource.instances.retain(|r| {
                let mut r = r.borrow_mut();
                match &*r {
                    PhysicalComponentState::Requested(_extra_constraints) => {
                        let dst = select_node_for_resource(&r_class_clone, nodes);
                        if let Some(dst) = dst {
                            *r = PhysicalComponentState::Materialized(Box::new(resource::PhysicalResource {
                                id: edgeless_api::function_instance::InstanceId::new(dst),
                                desired_mapping: PhysicalPorts::default(),
                                materialized: None,
                                creation_time: std::time::Instant::now(),
                                class: r_class_clone.clone(),
                                component_name: resource_id.clone(),
                                configuration: r_configuration_clone.clone(),
                            }));
                        } else {
                            return false;
                        }
                    }
                    _ => {
                        //NOOP
                    }
                }
                true
            });
        }

        for subflow in workflow.subflows.values_mut() {
            let subflow = subflow.borrow_mut();

            for s in &subflow.instances {
                let mut s = s.borrow_mut();
                match &*s {
                    PhysicalComponentState::Requested(_extra_constraints) => {
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
                    PhysicalComponentState::Requested(_extra_constraints) => {
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

impl<P: strategy::PlacementStrategy> DefaultPlacement<P> {
    fn process_actor(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        actor_id: &str,
        actor: &mut crate::ir::actor::LogicalActor,
        nodes: &crate::ir::Nodes,
        global_state: &PlacementState<P>,
    ) {
        let num_instances = actor.instances.len();
        let num_active_instances = actor.instances.iter().filter(|i| i.borrow().try_unpack_active().is_some()).count();

        let cloned_node_filters = actor.node_filter.clone();
        let cloned_function_instance_len = actor.instances.len();
        let cloned_init_on = actor.annotations.get("node_id_init_on").cloned();

        let cloned_function = actor.clone();

        global_state.image_chache.insert_blocking(actor.image.main_image.clone());
        for extra in actor.image.extra_images.clone() {
            tracing::debug!("Storing Extra Image in Cache: {:?}", extra.behavior_image_id);
            global_state.image_chache.insert_blocking(extra);
        }

        let mut new_instances = Vec::new();

        actor.instances.retain(|i| {
            let mut i = i.borrow_mut();
            match &mut *i {
                PhysicalComponentState::Requested(extra_constraints) => {
                    let mut node_filters = if let Some(extra_constraints) = extra_constraints {
                        extra_constraints.clone()
                    } else {
                        cloned_node_filters.clone()
                    };

                    // This was added for evaluation purposes
                    if let Some(dest_node) = &cloned_init_on {
                        if num_instances == 1 {
                            let dest_uuid = uuid::Uuid::from_str(dest_node).unwrap();
                            node_filters.node_ids_allowed = Some(vec![dest_uuid])
                        }
                    }

                    let placement_constraints = PlacementConstraints {
                        node_filters,
                        urgent: num_active_instances < 1,
                    };

                    let new_instance = self.spawn_new_actor(
                        workflow,
                        actor_id.to_string(),
                        &cloned_function,
                        nodes,
                        global_state,
                        &placement_constraints,
                        workflow.feature_flags.disable_actor_optimization,
                    );
                    if let Some(new_instance) = new_instance {
                        *i = new_instance;
                    } else {
                        tracing::info!(
                            "Requested Instance: Found no viable node for {} in {}",
                            &actor_id,
                            workflow.id.workflow_id
                        );
                        return false;
                    }
                }
                PhysicalComponentState::MigrationRequested(c) => {
                    let node_filters = cloned_node_filters.clone();

                    // This would allow to guarantee getting a different instance but that might be less efficient than the old instance.
                    // To properly do this, we might be required to also return the efficiency and compare it here.
                    // node_filters.node_ids_denied.get_or_insert_default().push(c.id().node_id.clone());

                    let placement_constraints = PlacementConstraints { node_filters, urgent: false };

                    let new_instance = self.spawn_new_actor(
                        workflow,
                        actor_id.to_string(),
                        &cloned_function,
                        nodes,
                        global_state,
                        &placement_constraints,
                        workflow.feature_flags.disable_actor_optimization,
                    );
                    if let Some(new_instance) = new_instance {
                        let new_id = new_instance.id().unwrap();
                        if new_id.node_id == c.id().node_id {
                            tracing::info!(
                                "Migrating Instance: Node would be equal {}({}). {}",
                                actor_id,
                                c.id(),
                                cloned_function_instance_len
                            );
                            i.abort_migration();
                        } else {
                            tracing::info!(
                                "MigratingInstance: Found Replacement node for {} in {} ({}); Will migrate: {} -> {}",
                                &actor_id,
                                workflow.id.workflow_id,
                                c.id(),
                                c.id().node_id,
                                new_id.node_id
                            );
                            new_instances.push(std::cell::RefCell::new(new_instance));
                            i.mark_migrating_away(new_id);
                        }
                    } else {
                        tracing::debug!("Migration: Could not spawn replacement instance. Aborting.");
                        i.abort_migration();
                    }
                }
                PhysicalComponentState::Lost(_) => match &cloned_function.scaling_mode {
                    crate::ir::component::ScalingMode::AllNodes => {
                        i.mark_stopped();
                    }
                    _ => {
                        let node_filters = cloned_node_filters.clone();
                        let placement_constraints = PlacementConstraints {
                            node_filters,
                            urgent: num_active_instances <= 1,
                        };

                        let new_instance = self.spawn_new_actor(
                            workflow,
                            actor_id.to_string(),
                            &cloned_function,
                            nodes,
                            global_state,
                            &placement_constraints,
                            workflow.feature_flags.disable_actor_optimization,
                        );
                        if let Some(new_instance) = new_instance {
                            let new_id = new_instance.id().unwrap();
                            new_instances.push(std::cell::RefCell::new(new_instance));
                            i.mark_lost_replaced(new_id);
                        }
                    }
                },
                PhysicalComponentState::Dead(old_instance) => {
                    let node_filters = match &cloned_function.scaling_mode {
                        crate::ir::component::ScalingMode::AllNodes => {
                            let mut filters = cloned_node_filters.clone();
                            filters.node_ids_allowed = Some(vec![old_instance.id().node_id.clone()]);
                            filters
                        }
                        _ => cloned_node_filters.clone(),
                    };

                    let placement_constraints = PlacementConstraints {
                        node_filters: node_filters,
                        urgent: num_active_instances <= 1,
                    };

                    let new_instance = self.spawn_new_actor(
                        workflow,
                        actor_id.to_string(),
                        &cloned_function,
                        nodes,
                        global_state,
                        &placement_constraints,
                        workflow.feature_flags.disable_actor_optimization,
                    );
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
            true
        });
        actor.instances.extend(new_instances);
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_new_actor(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        logical_name: String,
        actor: &crate::ir::actor::LogicalActor,
        nodes: &crate::ir::Nodes,
        global_state: &PlacementState<P>,
        placement_constraints: &PlacementConstraints,
        disable_actor_optimization: bool,
    ) -> Option<PhysicalComponentState> {
        let candidates = find_candidates_for_actor(placement_constraints, actor, nodes, global_state.image_chache, disable_actor_optimization);

        let mut filtered = self.dynamic_colocation_filter.filter_candidates(actor, candidates, workflow);

        if filtered.len() > 1 {
            filtered = self.static_colocation_filter.filter_candidates(actor, filtered, workflow);
        };

        let dst = self.placement_strategy.select_candidate(filtered, global_state.strategy_state);

        if let Some(dst) = dst {
            let new_id = edgeless_api::function_instance::InstanceId::new(dst.node_id);
            let new_instance = actor::PhysicalActor {
                id: new_id,
                desired_mapping: PhysicalPorts::default(),
                image: dst.dest_image,
                behavior_spec: actor.image.spec.clone(),
                materialized: None,
                creation_time: std::time::Instant::now(),
                component_name: logical_name,
                annotations: actor.annotations.clone(),
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
    placement_constraints: &PlacementConstraints,
    logical_actor: &crate::ir::actor::LogicalActor,
    nodes: &'b crate::ir::Nodes,
    image_cache: &crate::ir::support::image_cache::ImageCache,
    disable_actor_optimization: bool,
) -> Vec<Candidate<'b>> {
    let mut candiates = Vec::new();

    // For urgent requests (e.g. the first instance), attempt to use an existing image.
    // This can fail as the normal mode will always be executed if the urgen mode fails.
    if placement_constraints.urgent {
        tracing::debug!("Urgent Mode");
        for node in nodes.values() {
            let node_candidates = feasibility::feasible_node_runtime_candidates(
                &placement_constraints.node_filters,
                logical_actor,
                *node,
                true,
                disable_actor_optimization,
            );
            if let Some(node_candidate) = select_actor_node_candidate(node_candidates, true, true, image_cache) {
                candiates.push(node_candidate)
            }
        }

        if !candiates.is_empty() {
            tracing::debug!("Found 'Urgent' Image");
            return candiates;
        }
    }

    // Attempt to request the best image.
    for node in nodes.values() {
        let node_candidates = feasibility::feasible_node_runtime_candidates(
            &placement_constraints.node_filters,
            logical_actor,
            *node,
            false,
            disable_actor_optimization,
        );
        if let Some(node_candidate) = select_actor_node_candidate(node_candidates, false, false, image_cache) {
            candiates.push(node_candidate)
        }
    }

    if !candiates.is_empty() {
        return candiates;
    }

    // Last attempt: Just use any image
    for node in nodes.values() {
        let node_candidates = feasibility::feasible_node_runtime_candidates(
            &placement_constraints.node_filters,
            logical_actor,
            *node,
            true,
            disable_actor_optimization,
        );
        if let Some(node_candidate) = select_actor_node_candidate(node_candidates, false, true, image_cache) {
            candiates.push(node_candidate)
        }
    }

    if !candiates.is_empty() {
        tracing::debug!("Found 'Any' Image");
    }

    candiates
}

fn select_actor_node_candidate<'b>(
    candiates: Vec<Candidate<'b>>,
    only_available: bool,
    allow_imperfect: bool,
    image_cache: &crate::ir::support::image_cache::ImageCache,
) -> Option<Candidate<'b>> {
    let mut viable_candidates: Vec<_> = candiates
        .into_iter()
        .filter_map(|c| {
            let image_request = c.dest_image.clone();

            let image_ident = if let actor::ImageState::Planned(p) = image_request {
                p
            } else {
                return Some(c);
            };

            // We here assume the cache also contains the main image and all extra images.
            let existing = image_cache.get_blocking(&image_ident);

            match existing {
                image_cache::CacheResult::NotFound => {
                    tracing::debug!("No fully matching image found: {image_ident:?}.");
                    if !only_available {
                        tracing::debug!("Returning source image.");
                        return Some(c);
                    }
                }
                image_cache::CacheResult::PartialMatch(partial_match) => {
                    if allow_imperfect {
                        let mut image_options: Vec<_> = partial_match
                            .same_runtime_feature_subset(&image_ident)
                            .same_or_more_ports(&image_ident)
                            .into();
                        image_options.sort_by(|a, b| {
                            let a_diff = a
                                .behavior_image_id
                                .dialect_type
                                .features
                                .difference(&image_ident.dialect_type.features)
                                .count();
                            let b_diff = b
                                .behavior_image_id
                                .dialect_type
                                .features
                                .difference(&image_ident.dialect_type.features)
                                .count();
                            a_diff.cmp(&b_diff)
                        });
                        if let Some(image) = image_options.pop() {
                            let mut new_candidate = c.clone();
                            new_candidate.dest_image = actor::ImageState::Existing(image.clone());
                            return Some(new_candidate);
                        }
                    }
                    if !only_available {
                        tracing::debug!("Partial match failed; Returning source image.");
                        return Some(c);
                    }
                }
                image_cache::CacheResult::FullMatch(image) => {
                    let mut new_candidate = c.clone();
                    new_candidate.dest_image = actor::ImageState::Existing(image.clone());
                    return Some(new_candidate);
                }
            }
            None
        })
        .collect();
    viable_candidates.sort_by(|a, b| a.runtime.efficiency_score().total_cmp(&b.runtime.efficiency_score()));
    viable_candidates.pop()
}

fn select_node_for_resource(resource_class: &str, nodes: &crate::ir::Nodes) -> Option<edgeless_api::function_instance::NodeId> {
    if let Some((id, _)) = nodes
        .iter()
        .find(|(_, n)| n.available_resource_providers().iter().any(|(_, r)| r.class_type() == resource_class))
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
