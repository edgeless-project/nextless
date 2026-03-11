// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod candidate_filter;
pub mod feasibility;
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
    pub instance_counts: &'a InstanceCounts,
}

#[derive(Default, Clone)]
pub struct InstanceCounts {
    resource_instance_counts:
        std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<(uuid::Uuid, String), std::collections::BTreeSet<uuid::Uuid>>>>,
}

struct PlacementConstraints {
    node_filters: crate::ir::component::NodeFilters,
    /// Used to indicate that the system should prefer fast deployment over efficiency.
    urgent: bool,
}

#[derive(Clone)]
pub enum Candidate<'a> {
    Actor(ActorCandidate<'a>),
    Resource(ResourceCandidate),
}

#[derive(Clone)]
pub struct ActorCandidate<'a> {
    pub(crate) node_id: edgeless_api::function_instance::NodeId,
    pub(crate) dest_image: actor::ImageState,
    pub(crate) runtime: crate::ir::Runtime<'a>,
}

#[derive(Clone)]
pub struct ResourceCandidate {
    pub(crate) node_id: edgeless_api::function_instance::NodeId,
    pub(crate) provider_id: String,
}

impl<'a> Candidate<'a> {
    fn node_id(&'a self) -> edgeless_api::function_instance::NodeId {
        match self {
            Candidate::Actor(actor_candidate) => actor_candidate.node_id.clone(),
            Candidate::Resource(resource_candidate) => resource_candidate.node_id.clone(),
        }
    }
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
            required_changes.extend(self.process_component(workflow, f_id, function, instances, nodes, peer_clusters, global_state))
        }

        required_changes
    }

    fn apply_stop(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        global_state: &PlacementState<P>,
    ) -> Vec<super::PhysicalChange> {
        let mut required_changes = Vec::new();

        for (_, _, instances) in workflow.components_with_instances() {
            for instance in instances {
                if let Some(active_instance) = instance.component.try_unpack_active() {
                    required_changes.extend(instance.plan_stop());
                    if let Some(resource_instance) = active_instance.as_resource() {
                        global_state
                            .instance_counts
                            .resource_instance_counts
                            .blocking_lock()
                            .entry((resource_instance.id.node_id.clone(), resource_instance.provider.clone()))
                            .and_modify(|e| {
                                e.remove(&resource_instance.id.function_id);
                            });
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
        logical_component: &crate::ir::logical_model::LogicalComponent,
        component_instances: crate::ir::workflow::InstanceIterator,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &PlacementState<P>,
    ) -> Vec<super::PhysicalChange> {
        tracing::debug!(
            "Component Instances {logical_component_id}: {:?}",
            component_instances.clone().map(|p| p.component_id).collect::<Vec<_>>()
        );
        let num_instances = component_instances.clone().count();
        let num_active_instances = component_instances.clone().filter_active().count();

        let cloned_init_on = logical_component.node_filters().node_id_init_on.clone();

        if let crate::ir::logical_model::LogicalComponent::Actor(logical_actor) = logical_component {
            global_state.image_chache.insert_blocking(logical_actor.image.main_image.clone());
            for extra in logical_actor.image.extra_images.clone() {
                tracing::debug!("Storing Extra Image in Cache: {:?}", extra.behavior_image_id);
                global_state.image_chache.insert_blocking(extra);
            }
        }

        let mut required_changes = Vec::new();

        for i in component_instances {
            match &*i.component {
                PhysicalComponentState::Requested(extra_constraints) => {
                    let mut node_filters = if let Some(extra_constraints) = extra_constraints {
                        extra_constraints.clone()
                    } else {
                        logical_component.node_filters()
                    };

                    // This was added for evaluation purposes
                    if let Some(dest_node) = &cloned_init_on {
                        if num_instances == 1 {
                            let dest_uuid = dest_node.clone();
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
                        &logical_component,
                        nodes,
                        peer_clusters,
                        global_state,
                        &placement_constraints,
                        workflow.feature_flags.disable_actor_optimization,
                    );
                    if let Some(new_instance) = new_instance {
                        if let Some(resource_instance) = new_instance.try_unpack_active().and_then(|i| i.as_resource()) {
                            global_state
                                .instance_counts
                                .resource_instance_counts
                                .blocking_lock()
                                .entry((resource_instance.id.node_id.clone(), resource_instance.provider.clone()))
                                .or_default()
                                .insert(resource_instance.id.function_id);
                        }
                        required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
                            component_id: i.component_id,
                            action: super::PhysicalComponentChangeAction::Update(new_instance),
                        }));
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
                    let node_filters = logical_component.node_filters();

                    // This would allow to guarantee getting a different instance but that might be less efficient than the old instance.
                    // To properly do this, we might be required to also return the efficiency and compare it here.
                    // node_filters.node_ids_denied.get_or_insert_default().push(c.id().node_id.clone());

                    let placement_constraints = PlacementConstraints { node_filters, urgent: false };

                    let new_component_id = uuid::Uuid::new_v4();
                    let new_instance = self.spawn_new_component_instance(
                        workflow,
                        new_component_id,
                        logical_component_id.to_string(),
                        &logical_component,
                        nodes,
                        peer_clusters,
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
                            // This is wrong/suboptimal here, but doing this properly would require sharing the counts between stages.
                            if let Some(resource_instance) = i.component.try_unpack_active().and_then(|i| i.as_resource()) {
                                global_state
                                    .instance_counts
                                    .resource_instance_counts
                                    .blocking_lock()
                                    .entry((resource_instance.id.node_id.clone(), resource_instance.provider.clone()))
                                    .and_modify(|e| {
                                        e.remove(&resource_instance.id.function_id);
                                    });
                            }
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
                PhysicalComponentState::Lost(_) => {
                    if let Some(resource_instance) = i.component.try_unpack_active().and_then(|i| i.as_resource()) {
                        global_state
                            .instance_counts
                            .resource_instance_counts
                            .blocking_lock()
                            .entry((resource_instance.id.node_id.clone(), resource_instance.provider.clone()))
                            .and_modify(|e| {
                                e.remove(&resource_instance.id.function_id);
                            });
                    }
                    match &logical_component.scaling_mode() {
                        crate::ir::component::ScalingMode::AllNodes => {
                            required_changes.extend(i.mark_stopped());
                        }
                        _ => {
                            let node_filters = logical_component.node_filters();
                            let placement_constraints = PlacementConstraints {
                                node_filters,
                                urgent: num_active_instances <= 1,
                            };

                            let new_component_id = uuid::Uuid::new_v4();
                            let new_instance = self.spawn_new_component_instance(
                                workflow,
                                new_component_id,
                                logical_component_id.to_string(),
                                &logical_component,
                                nodes,
                                peer_clusters,
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
                    }
                }
                PhysicalComponentState::Dead(old_instance) => {
                    if let Some(resource_instance) = i.component.try_unpack_active().and_then(|i| i.as_resource()) {
                        global_state
                            .instance_counts
                            .resource_instance_counts
                            .blocking_lock()
                            .entry((resource_instance.id.node_id.clone(), resource_instance.provider.clone()))
                            .and_modify(|e| {
                                e.remove(&resource_instance.id.function_id);
                            });
                    }

                    let node_filters = match &logical_component.scaling_mode() {
                        crate::ir::component::ScalingMode::AllNodes => {
                            let mut filters = logical_component.node_filters();
                            filters.node_ids_allowed = Some(vec![old_instance.id().node_id.clone()]);
                            filters
                        }
                        _ => logical_component.node_filters(),
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
                        &logical_component,
                        nodes,
                        peer_clusters,
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
        peer_clusters: &crate::ir::Clusters,
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
            LogicalComponent::Resource(logical_resource) => {
                find_candidates_for_resource(placement_constraints, logical_resource, nodes, global_state.instance_counts)
            }
            LogicalComponent::SubApplication(_logical_sub_flow) => {
                tracing::warn!("Tried to place SubApplication");
                tracing::warn!("Peer Clusters: {:?}", peer_clusters.keys());
                vec![]
            }
            LogicalComponent::Proxy(_logical_proxy) => {
                tracing::warn!("Tried to place Proxy");
                vec![]
            }
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
                node_id: dst.node_id(),
            };

            let new_instance: Box<dyn PhysicalComponent> = match component {
                LogicalComponent::Actor(logical_actor) => {
                    let Candidate::Actor(actor_candidate) = dst else { return None };
                    let actor = actor::PhysicalActor {
                        id: new_id,
                        desired_mapping: PhysicalPorts::default(),
                        image: actor_candidate.dest_image,
                        behavior_spec: logical_actor.image.spec.clone(),
                        materialized: None,
                        creation_time: std::time::Instant::now(),
                        component_name: logical_name,
                        annotations: logical_actor.annotations.clone(),
                    };
                    Box::new(actor)
                }
                LogicalComponent::Resource(logical_resource) => {
                    let Candidate::Resource(resource_candidate) = dst else { return None };
                    let resource = resource::PhysicalResource {
                        id: new_id,
                        desired_mapping: PhysicalPorts::default(),
                        materialized: None,
                        creation_time: std::time::Instant::now(),
                        class: logical_resource.class.clone(),
                        component_name: logical_name.to_string(),
                        configuration: logical_resource.configurations.clone(),
                        provider: resource_candidate.provider_id.clone(),
                    };
                    Box::new(resource)
                }
                LogicalComponent::SubApplication(_logical_sub_flow) => {
                    let subflow = subflow::PhysicalSubFlow {
                        id: new_id,
                        desired_mapping: PhysicalPorts::default(),
                        materialized: None,
                        creation_time: std::time::Instant::now(),
                        internal_ports: InternalPorts::default(),
                    };
                    Box::new(subflow)
                }
                LogicalComponent::Proxy(_logical_proxy) => {
                    let proxy = proxy::PhyiscalProxy {
                        id: new_id,
                        desired_mapping: PhysicalPorts::default(),
                        materialized: None,
                        creation_time: std::time::Instant::now(),
                        external_ports: ExternalPorts::default(),
                    };
                    Box::new(proxy)
                }
            };

            let (_, empty_component_state) = PhysicalComponentState::request_new_instance();
            empty_component_state.plan_creation(new_instance).map(|x| x)
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
            return candiates.into_iter().map(|c| Candidate::Actor(c)).collect();
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
        return candiates.into_iter().map(|c| Candidate::Actor(c)).collect();
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

    candiates.into_iter().map(|c| Candidate::Actor(c)).collect()
}

fn find_candidates_for_resource<'b>(
    placement_constraints: &PlacementConstraints,
    logical_resource: &crate::ir::resource::LogicalResource,
    nodes: &'b crate::ir::Nodes,
    instance_counts: &'b InstanceCounts,
) -> Vec<Candidate<'b>> {
    let mut candidates = Vec::new();

    for (_node_id, node) in nodes {
        for candidate in feasibility::node_can_host_resource(&placement_constraints.node_filters, logical_resource, *node, instance_counts) {
            candidates.push(Candidate::Resource(candidate));
        }
    }

    candidates
}

fn select_actor_node_candidate<'b>(
    candiates: Vec<ActorCandidate<'b>>,
    only_available: bool,
    allow_imperfect: bool,
    image_cache: &crate::ir::support::image_cache::ImageCache,
) -> Option<ActorCandidate<'b>> {
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

#[cfg(test)]
mod test {
    use crate::ir::transformations::{placement::strategy::PlacementStrategy, StatefulPhysicalTransformation};

    struct MockResourceProvider {
        instance_limit: Option<usize>,
    }

    impl crate::ir::ResourceProvider for MockResourceProvider {
        fn class_type(&self) -> String {
            "test-class".to_string()
        }

        fn outputs(&self) -> Vec<String> {
            vec![]
        }

        fn instance_limit(&self) -> Option<usize> {
            self.instance_limit.clone()
        }
    }

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

        let changes = run_transformation(&workflow, &nodes, None);

        assert_eq!(changes.len(), 1);

        let super::transformations::PhysicalChange::Component(component_change) = &changes[0] else {
            panic!("Unexpected Physical Change");
        };

        assert!(component_change.component_id == actor_instance_id);

        let super::transformations::PhysicalComponentChangeAction::Update(update) = &component_change.action else {
            panic!("Unexpected Change Action");
        };

        let crate::ir::physical_model::PhysicalComponentState::Planned(_) = update else {
            panic!("Unexpected State Change");
        };
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn limited_resource_placement_and_stop() {
        let (resource_id, logical_resource) = crate::ir::resource::mock_resource::MockResourceBuilder::default()
            .with_class("test-class")
            .build();
        let (_, resource_instance_id, resource_instance) =
            crate::ir::resource::mock_resource::MockResourceInstanceBuilder::new_for_logical(&resource_id, &logical_resource)
                .with_state(crate::ir::actor::mock_actor::DesiredPhysicalComponentState::Requested)
                .build();

        let mut workflow = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&resource_id, &logical_resource, &[(resource_instance_id, resource_instance)])
            .build();

        let example_node_id = uuid::Uuid::new_v4();

        let mock_resource_provider = MockResourceProvider { instance_limit: Some(1) };

        let mock_node = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id.clone())
            .resource_providers(crate::ir::ResourceProviders::from([(
                "test-provider-1".to_string(),
                &mock_resource_provider as &dyn crate::ir::ResourceProvider,
            )]))
            .build()
            .unwrap();

        let nodes = std::collections::HashMap::from([(example_node_id.clone(), &mock_node as &dyn crate::ir::Node)]);

        let instance_counts = crate::ir::transformations::placement::InstanceCounts::default();

        // Placement
        {
            let changes = run_transformation(&workflow, &nodes, Some(instance_counts.clone()));

            assert_eq!(changes.len(), 1);

            let super::transformations::PhysicalChange::Component(component_change) = &changes[0] else {
                panic!("Unexpected Physical Change");
            };

            assert!(component_change.component_id == resource_instance_id);

            let super::transformations::PhysicalComponentChangeAction::Update(update) = &component_change.action else {
                panic!("Unexpected Change Action");
            };

            let crate::ir::physical_model::PhysicalComponentState::Planned(_) = update else {
                panic!("Unexpected State Change");
            };

            assert_eq!(
                instance_counts
                    .resource_instance_counts
                    .lock()
                    .await
                    .get(&(example_node_id.clone(), "test-provider-1".to_string()))
                    .unwrap()
                    .len(),
                1
            );
            workflow.apply_physical_changes(changes);
        }

        // Mark All Instances Started
        {
            let mut all_changes = vec![];
            for (_id, _component, instances) in workflow.components_with_instances() {
                for instance in instances {
                    let super::physical_model::PhysicalComponentState::Planned(planned_instance) = instance.component else {
                        panic!("Unexpected State");
                    };

                    let (changes, _) = planned_instance.materialize(&None);
                    all_changes.extend(changes);
                }
            }
            workflow.apply_physical_changes(all_changes);
        }

        // Stop
        {
            let changes = run_stop(&workflow, &nodes, Some(instance_counts.clone()));

            assert_eq!(changes.len(), 1);

            let super::transformations::PhysicalChange::Component(component_change) = &changes[0] else {
                panic!("Unexpected Physical Change");
            };

            assert!(component_change.component_id == resource_instance_id);

            let super::transformations::PhysicalComponentChangeAction::Update(update) = &component_change.action else {
                panic!("Unexpected Change Action");
            };

            let crate::ir::physical_model::PhysicalComponentState::StopPlanned { .. } = update else {
                panic!("Unexpected State Change");
            };

            assert!(component_change.component_id == resource_instance_id);

            assert_eq!(
                instance_counts
                    .resource_instance_counts
                    .lock()
                    .await
                    .get(&(example_node_id.clone(), "test-provider-1".to_string()))
                    .unwrap()
                    .len(),
                0
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn dont_place_if_resource_limit_exceeded() {
        let (resource_id, logical_resource) = crate::ir::resource::mock_resource::MockResourceBuilder::default()
            .with_class("test-class")
            .build();
        let (_, resource_instance_id, resource_instance) =
            crate::ir::resource::mock_resource::MockResourceInstanceBuilder::new_for_logical(&resource_id, &logical_resource)
                .with_state(crate::ir::actor::mock_actor::DesiredPhysicalComponentState::Requested)
                .build();

        let workflow = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&resource_id, &logical_resource, &[(resource_instance_id, resource_instance)])
            .build();

        let example_node_id = uuid::Uuid::new_v4();

        let mock_resource_provider = MockResourceProvider { instance_limit: Some(1) };

        let mock_node = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id.clone())
            .resource_providers(crate::ir::ResourceProviders::from([(
                "test-provider-1".to_string(),
                &mock_resource_provider as &dyn crate::ir::ResourceProvider,
            )]))
            .build()
            .unwrap();

        let nodes = std::collections::HashMap::from([(example_node_id.clone(), &mock_node as &dyn crate::ir::Node)]);

        let instance_counts = crate::ir::transformations::placement::InstanceCounts::default();

        instance_counts.resource_instance_counts.lock().await.insert(
            (example_node_id.clone(), "test-provider-1".to_string()),
            std::collections::BTreeSet::from([uuid::Uuid::new_v4()]),
        );

        let changes = run_transformation(&workflow, &nodes, Some(instance_counts));

        assert_eq!(changes.len(), 1);

        let super::transformations::PhysicalChange::Component(component_change) = &changes[0] else {
            panic!("Unexpected PhysicalChange");
        };

        assert!(component_change.component_id == resource_instance_id);

        let super::transformations::PhysicalComponentChangeAction::Delete = &component_change.action else {
            panic!("Unexpected Change Action");
        };
    }

    fn run_transformation(
        workflow: &crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        instance_counts: Option<crate::ir::transformations::placement::InstanceCounts>,
    ) -> Vec<super::transformations::PhysicalChange> {
        let mut transformation = super::DefaultPlacement::<super::strategy::random::Random>::new(super::strategy::random::Random::new());

        let image_cache = crate::ir::support::image_cache::ImageCache::new();
        let instance_counts = instance_counts.unwrap_or(crate::ir::transformations::placement::InstanceCounts::default());

        let placement_state = super::PlacementState {
            strategy_state: &(),
            image_chache: &image_cache,
            instance_counts: &instance_counts,
        };

        tokio::task::block_in_place(|| transformation.apply(workflow, &nodes, &Default::default(), &placement_state))
    }

    fn run_stop(
        workflow: &crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        instance_counts: Option<crate::ir::transformations::placement::InstanceCounts>,
    ) -> Vec<super::transformations::PhysicalChange> {
        let mut transformation = super::DefaultPlacement::<super::strategy::random::Random>::new(super::strategy::random::Random::new());

        let image_cache = crate::ir::support::image_cache::ImageCache::new();
        let instance_counts = instance_counts.unwrap_or(crate::ir::transformations::placement::InstanceCounts::default());

        let placement_state = super::PlacementState {
            strategy_state: &(),
            image_chache: &image_cache,
            instance_counts: &instance_counts,
        };

        tokio::task::block_in_place(|| transformation.apply_stop(workflow, &nodes, &Default::default(), &placement_state))
    }
}
