// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod candidate_filter;
pub mod feasibility;
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

impl<'a, P: strategy::PlacementStrategy> super::StatefulPhysicalTransformation<PlacementState<'a, P>> for DefaultPlacement<P> {
    #[tracing::instrument(name = "placement", skip_all)]
    fn apply(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &PlacementState<P>,
    ) -> Vec<super::PhysicalChange> {
        let mut required_changes = Vec::new();

        for (f_id, function, instances) in workflow.components_with_instances() {
            match function {
                LogicalComponent::Actor(_logical_actor) => {
                    required_changes.extend(self.process_component(workflow, f_id, function, instances, nodes, global_state))
                }
                LogicalComponent::Resource(logical_resource) => {
                    let r_class_clone = logical_resource.class.to_string();
                    let r_configuration_clone = logical_resource.configurations.clone();

                    for resource_instance in instances {
                        match &*resource_instance.component {
                            PhysicalComponentState::Requested(_extra_constraints) => {
                                let dst = select_node_for_resource(&r_class_clone, nodes);
                                if let Some(dst) = dst {
                                    let instance = PhysicalComponentState::Materialized(Box::new(resource::PhysicalResource {
                                        id: edgeless_api::function_instance::InstanceId {
                                            node_id: dst,
                                            function_id: resource_instance.component_id,
                                        },
                                        desired_mapping: PhysicalPorts::default(),
                                        materialized: None,
                                        creation_time: std::time::Instant::now(),
                                        class: r_class_clone.clone(),
                                        component_name: f_id.to_string(),
                                        configuration: r_configuration_clone.clone(),
                                    }));
                                    required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
                                        component_id: resource_instance.component_id,
                                        action: super::PhysicalComponentChangeAction::Update(instance),
                                    }));
                                } else {
                                    required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
                                        component_id: resource_instance.component_id,
                                        action: super::PhysicalComponentChangeAction::Delete,
                                    }));
                                }
                            }
                            _ => {
                                //NOOP
                            }
                        }
                    }
                }
                LogicalComponent::SubApplication(logical_sub_flow) => {
                    for s in instances {
                        match &*s.component {
                            PhysicalComponentState::Requested(_extra_constraints) => {
                                let dst = select_cluster_for_subflow(&logical_sub_flow, peer_clusters);
                                if let Some(dst) = dst {
                                    let subflow_instance = PhysicalComponentState::Materialized(Box::new(subflow::PhysicalSubFlow {
                                        id: edgeless_api::function_instance::InstanceId {
                                            node_id: dst,
                                            function_id: s.component_id,
                                        },
                                        desired_mapping: PhysicalPorts::default(),
                                        materialized: None,
                                        creation_time: std::time::Instant::now(),
                                        internal_ports: InternalPorts::default(),
                                    }));

                                    required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
                                        component_id: s.component_id,
                                        action: super::PhysicalComponentChangeAction::Update(subflow_instance),
                                    }));
                                }
                            }
                            _ => {
                                //NOOP
                            }
                        }
                    }
                }
                LogicalComponent::Proxy(logical_proxy) => {
                    for p in instances {
                        match p.component {
                            PhysicalComponentState::Requested(_extra_constraints) => {
                                let dst = select_node_for_proxy(logical_proxy, nodes);
                                if let Some(dst) = dst {
                                    let proxy_instance = PhysicalComponentState::Materialized(Box::new(proxy::PhyiscalProxy {
                                        id: edgeless_api::function_instance::InstanceId {
                                            node_id: dst,
                                            function_id: p.component_id,
                                        },
                                        desired_mapping: PhysicalPorts::default(),
                                        materialized: None,
                                        creation_time: std::time::Instant::now(),
                                        external_ports: ExternalPorts::default(),
                                    }));

                                    required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
                                        component_id: p.component_id,
                                        action: super::PhysicalComponentChangeAction::Update(proxy_instance),
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

        required_changes
    }
}

impl<P: strategy::PlacementStrategy> DefaultPlacement<P> {
    fn process_component(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        logical_component_id: &str,
        actor: &crate::ir::logical_model::LogicalComponent,
        actor_instances: crate::ir::workflow::InstanceIterator,
        nodes: &crate::ir::Nodes,
        global_state: &PlacementState<P>,
    ) -> Vec<super::PhysicalChange> {
        tracing::warn!("{:?}", actor_instances.clone().map(|p| p.component_id).collect::<Vec<_>>());
        let num_instances = actor_instances.clone().count();
        let num_active_instances = actor_instances.clone().filter_active().count();

        let cloned_init_on = if let crate::ir::logical_model::LogicalComponent::Actor(logical_actor) = actor {
            logical_actor.annotations.get("node_id_init_on").cloned()
        } else {
            None
        };

        if let crate::ir::logical_model::LogicalComponent::Actor(logical_actor) = actor {
            global_state.image_chache.insert_blocking(logical_actor.image.main_image.clone());
            for extra in logical_actor.image.extra_images.clone() {
                tracing::debug!("Storing Extra Image in Cache: {:?}", extra.behavior_image_id);
                global_state.image_chache.insert_blocking(extra);
            }
        }

        let mut required_changes = Vec::new();

        for i in actor_instances {
            // let mut i = i.borrow_mut();
            match &*i.component {
                PhysicalComponentState::Requested(extra_constraints) => {
                    let mut node_filters = if let Some(extra_constraints) = extra_constraints {
                        extra_constraints.clone()
                    } else {
                        actor.node_filters()
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

                    let new_instance = self.spawn_new_component_instance(
                        workflow,
                        i.component_id,
                        logical_component_id.to_string(),
                        &actor,
                        nodes,
                        global_state,
                        &placement_constraints,
                        workflow.feature_flags.disable_actor_optimization,
                    );
                    if let Some(new_instance) = new_instance {
                        required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
                            component_id: i.component_id,
                            action: super::PhysicalComponentChangeAction::Update(new_instance),
                        }));
                        tracing::info!("Spawn worked");
                    } else {
                        tracing::debug!(
                            "Requested Instance: Found no viable node for {} in {}",
                            &logical_component_id,
                            workflow.id.workflow_id
                        );
                        required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
                            component_id: i.component_id,
                            action: super::PhysicalComponentChangeAction::Delete,
                        }));
                    }
                }
                PhysicalComponentState::MigrationRequested(c) => {
                    let node_filters = actor.node_filters();

                    // This would allow to guarantee getting a different instance but that might be less efficient than the old instance.
                    // To properly do this, we might be required to also return the efficiency and compare it here.
                    // node_filters.node_ids_denied.get_or_insert_default().push(c.id().node_id.clone());

                    let placement_constraints = PlacementConstraints { node_filters, urgent: false };

                    let new_component_id = uuid::Uuid::new_v4();
                    let new_instance = self.spawn_new_component_instance(
                        workflow,
                        new_component_id,
                        logical_component_id.to_string(),
                        &actor,
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
                                logical_component_id,
                                c.id(),
                                num_instances
                            );
                            required_changes.extend(i.abort_migration());
                        } else {
                            tracing::info!(
                                "MigratingInstance: Found Replacement node for {} in {} ({}); Will migrate: {} -> {}",
                                &logical_component_id,
                                workflow.id.workflow_id,
                                c.id(),
                                c.id().node_id,
                                new_id.node_id
                            );
                            required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
                                component_id: new_component_id,
                                action: super::PhysicalComponentChangeAction::Update(new_instance),
                            }));
                            required_changes.extend(i.mark_migrating_away(new_id));
                        }
                    } else {
                        tracing::debug!("Migration: Could not spawn replacement instance. Aborting.");
                        required_changes.extend(i.abort_migration());
                    }
                }
                PhysicalComponentState::Lost(_) => match &actor.scaling_mode() {
                    crate::ir::component::ScalingMode::AllNodes => {
                        required_changes.extend(i.mark_stopped());
                    }
                    _ => {
                        let node_filters = actor.node_filters();
                        let placement_constraints = PlacementConstraints {
                            node_filters,
                            urgent: num_active_instances <= 1,
                        };

                        let new_component_id = uuid::Uuid::new_v4();
                        let new_instance = self.spawn_new_component_instance(
                            workflow,
                            new_component_id,
                            logical_component_id.to_string(),
                            &actor,
                            nodes,
                            global_state,
                            &placement_constraints,
                            workflow.feature_flags.disable_actor_optimization,
                        );
                        if let Some(new_instance) = new_instance {
                            let new_id = new_instance.id().unwrap();
                            required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
                                component_id: new_component_id,
                                action: super::PhysicalComponentChangeAction::Update(new_instance),
                            }));
                            required_changes.extend(i.mark_lost_replaced(new_id));
                        }
                    }
                },
                PhysicalComponentState::Dead(old_instance) => {
                    let node_filters = match &actor.scaling_mode() {
                        crate::ir::component::ScalingMode::AllNodes => {
                            let mut filters = actor.node_filters();
                            filters.node_ids_allowed = Some(vec![old_instance.id().node_id.clone()]);
                            filters
                        }
                        _ => actor.node_filters(),
                    };

                    let placement_constraints = PlacementConstraints {
                        node_filters: node_filters,
                        urgent: num_active_instances <= 1,
                    };

                    let new_component_id = uuid::Uuid::new_v4();
                    let new_instance = self.spawn_new_component_instance(
                        workflow,
                        new_component_id,
                        logical_component_id.to_string(),
                        &actor,
                        nodes,
                        global_state,
                        &placement_constraints,
                        workflow.feature_flags.disable_actor_optimization,
                    );
                    if let Some(new_instance) = new_instance {
                        let new_id = new_instance.id().unwrap();
                        required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
                            component_id: new_component_id,
                            action: super::PhysicalComponentChangeAction::Update(new_instance),
                        }));
                        required_changes.extend(i.mark_dead_replaced(new_id));
                    }
                }
                _ => {
                    //NOOP
                }
            }
        }
        required_changes
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_new_component_instance(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        component_id: uuid::Uuid,
        logical_name: String,
        component: &crate::ir::logical_model::LogicalComponent,
        nodes: &crate::ir::Nodes,
        global_state: &PlacementState<P>,
        placement_constraints: &PlacementConstraints,
        disable_actor_optimization: bool,
    ) -> Option<PhysicalComponentState> {
        let candidates = match component {
            LogicalComponent::Actor(logical_actor) => find_candidates_for_actor(
                placement_constraints,
                logical_actor,
                nodes,
                global_state.image_chache,
                disable_actor_optimization,
            ),
            LogicalComponent::Resource(logical_resource) => todo!(),
            LogicalComponent::SubApplication(logical_sub_flow) => todo!(),
            LogicalComponent::Proxy(logical_proxy) => todo!(),
        };

        let mut filtered = self
            .dynamic_colocation_filter
            .filter_candidates(logical_name.clone(), component, candidates, workflow);

        if filtered.len() > 1 {
            filtered = self
                .static_colocation_filter
                .filter_candidates(logical_name.clone(), component, filtered, workflow);
        };

        let dst = self.placement_strategy.select_candidate(filtered, global_state.strategy_state);

        if let Some(dst) = dst {
            let new_id = edgeless_api::function_instance::InstanceId {
                function_id: component_id,
                node_id: dst.node_id,
            };

            let new_instance = match component {
                LogicalComponent::Actor(logical_actor) => actor::PhysicalActor {
                    id: new_id,
                    desired_mapping: PhysicalPorts::default(),
                    image: dst.dest_image,
                    behavior_spec: logical_actor.image.spec.clone(),
                    materialized: None,
                    creation_time: std::time::Instant::now(),
                    component_name: logical_name,
                    annotations: logical_actor.annotations.clone(),
                },
                LogicalComponent::Resource(logical_resource) => todo!(),
                LogicalComponent::SubApplication(logical_sub_flow) => todo!(),
                LogicalComponent::Proxy(logical_proxy) => todo!(),
            };

            let (_, empty_component_state) = PhysicalComponentState::request_new_instance();
            empty_component_state.plan_creation(Box::new(new_instance)).map(|x| x)
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

#[cfg(test)]
mod test {
    use crate::ir::transformations::{placement::strategy::PlacementStrategy, StatefulPhysicalTransformation};

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn place_actor_on_single_node() {
        let (actor_id, logical_actor) = crate::ir::actor::mock_actor::MockActorBuilder::default().build();
        let (_, actor_instance_id, actor_instance) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&actor_id, &logical_actor)
                .with_state(crate::ir::actor::mock_actor::DesiredPhysicalComponentState::Requested)
                .build();

        let workflow = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&actor_id, &logical_actor, &[(actor_instance_id, actor_instance)])
            .build();

        let example_node_id = uuid::Uuid::new_v4();

        let mock_runtime = crate::ir::test::MockWasmRuntime {};
        let mock_node = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id.clone())
            .runtimes(crate::ir::Runtimes::from([(
                "WASM".to_string(),
                crate::ir::Runtime::WasmBase(&mock_runtime, Default::default()),
            )]))
            .build()
            .unwrap();

        let nodes = std::collections::HashMap::from([(example_node_id.clone(), &mock_node as &dyn crate::ir::Node)]);

        let mut transformation = super::DefaultPlacement::<super::strategy::random::Random>::new(super::strategy::random::Random::new());

        let image_cache = crate::ir::support::image_cache::ImageCache::new();

        let placement_state = super::PlacementState {
            strategy_state: &(),
            image_chache: &image_cache,
        };

        let changes = tokio::task::block_in_place(|| transformation.apply(&workflow, &nodes, &Default::default(), &placement_state));

        assert_eq!(changes.len(), 1);
    }
}
