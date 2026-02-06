// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::pipeline::TransformationPipeline;

pub struct ManagedWorkflow<P: super::transformations::placement::strategy::PlacementStrategy> {
    pub wf: super::workflow::ActiveWorkflow,
    pub pipeline: super::pipeline::default::DefaultTransformationPipeline<P>,
    pub telemetry_provider: Option<Box<dyn super::TelemetryProvider>>,
}

impl<P: super::transformations::placement::strategy::PlacementStrategy> ManagedWorkflow<P> {
    pub fn new(
        request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
        id: edgeless_api::workflow_instance::WorkflowId,
        cluster_id: uuid::Uuid,
        telementry_provider: Option<Box<dyn super::TelemetryProvider>>,
        placement_strategy: P,
    ) -> Self {
        Self {
            wf: super::workflow::ActiveWorkflow::new(request, id, cluster_id),
            pipeline: super::pipeline::default::DefaultTransformationPipeline::new_default(placement_strategy),
            telemetry_provider: telementry_provider,
        }
    }

    #[tracing::instrument(name = "engine_initial_spawn", skip_all, fields(workflow_id = self.wf.id.to_string()))]
    pub fn initial_spawn(
        &mut self,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &super::pipeline::default::DefaultTransformationPipelineState<P::GlobalState>,
    ) -> Vec<super::RequiredChange> {
        tracing::info!("Initial Spawn");
        self.pipeline.apply_all(&mut self.wf, nodes, peer_clusters, global_state);
        self.materialize()
    }

    #[tracing::instrument(name = "engine_periodic_optimize", skip_all, fields(workflow_id = self.wf.id.to_string()))]
    pub fn periodic_optimize(
        &mut self,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &super::pipeline::default::DefaultTransformationPipelineState<P::GlobalState>,
    ) -> Vec<super::RequiredChange> {
        self.pipeline.apply_dynamic(&mut self.wf, nodes, peer_clusters, global_state);
        self.materialize()
    }

    #[tracing::instrument(name = "engine_node_removal", skip_all, fields(workflow_id = self.wf.id.to_string()))]
    pub fn node_removal(
        &mut self,
        removed_node_ids: &std::collections::HashSet<edgeless_api::function_instance::NodeId>,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &super::pipeline::default::DefaultTransformationPipelineState<P::GlobalState>,
    ) -> Vec<super::RequiredChange> {
        if self.remove_nodes(removed_node_ids) {
            self.pipeline.apply_dynamic(&mut self.wf, nodes, peer_clusters, global_state);
            self.materialize()
        } else {
            Vec::new()
        }
    }

    #[tracing::instrument(name = "engine_patch_external_links", skip_all, fields(workflow_id = self.wf.id.to_string()))]
    pub fn patch_external_links(
        &mut self,
        update: edgeless_api::common::PatchRequest,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &super::pipeline::default::DefaultTransformationPipelineState<P::GlobalState>,
    ) -> Vec<super::RequiredChange> {
        {
            let mut prx = self.wf.proxy.borrow_mut();
            prx.external_ports.external_input_mapping = crate::ir::physical_model::parse_api_input_mapping(update.input_mapping);
            prx.external_ports.external_output_mapping = crate::ir::physical_model::parse_api_output_mapping(update.output_mapping);
        }
        self.pipeline.apply_dynamic(&mut self.wf, nodes, peer_clusters, global_state);
        self.materialize()
    }

    #[allow(unused)]
    pub fn peer_cluster_removal(&self, _removed_cluster_ids: edgeless_api::function_instance::NodeId) -> Vec<super::RequiredChange> {
        Vec::new()
    }

    pub fn stop(&mut self) -> Vec<super::RequiredChange> {
        for (_, component_state) in self.wf.components() {
            let component_state = component_state.borrow();
            for instance in &mut component_state.instances() {
                let mut instance = instance.borrow_mut();
                if instance.try_unpack_active().is_some() {
                    instance.plan_stop();
                }
            }
        }

        self.materialize()
    }

    #[tracing::instrument(name = "calculate_required_changes", skip_all)]
    fn materialize(&mut self) -> Vec<super::RequiredChange> {
        let mut changes = Vec::new();

        tracing::debug!("Number of Links: {}", self.wf.links.len());

        for (link_id, link) in &mut self.wf.links {
            if !link.materialized {
                changes.push(super::RequiredChange::InstantiateLinkControlPlane {
                    link_id: link_id.clone(),
                    class: link.class.clone(),
                });
                link.materialized = true;
            }

            for (node, link_provider_id, node_config, node_materialized) in &mut link.nodes {
                if !*node_materialized {
                    changes.push(super::RequiredChange::CreateLinkOnNode {
                        node_id: *node,
                        provider_id: link_provider_id.clone(),
                        link_id: link_id.clone(),
                        config: node_config.clone(),
                    });
                    *node_materialized = true;
                }
            }
        }

        tracing::debug!("Number of Components: {}", self.wf.components().len());

        for (_c_name, function) in self.wf.components() {
            let function = function.borrow_mut();
            tracing::debug!("Number of Instances: {}", function.instances().len());
            for i in function.instances().iter() {
                let mut current = i.borrow_mut();
                match &mut *current {
                    super::PhysicalComponentState::Planned(planned_instance) => {
                        changes.extend(planned_instance.materialize(&self.telemetry_provider));
                        current.mark_materialized();
                    }
                    super::PhysicalComponentState::Materialized(maybe_dirty_instance) => {
                        changes.extend(maybe_dirty_instance.materialize(&self.telemetry_provider));
                    }
                    super::PhysicalComponentState::StopPlanned { old, .. } => {
                        changes.extend(old.stop());
                        current.mark_stopped();
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
        }

        changes
    }

    fn remove_nodes(&mut self, node_ids: &std::collections::HashSet<edgeless_api::function_instance::NodeId>) -> bool {
        let mut changed = false;
        for (_, component_state) in self.wf.components() {
            let component_state = component_state.borrow();
            for instance in &mut component_state.instances() {
                let mut instance = instance.borrow_mut();
                if let super::PhysicalComponentState::Materialized(component_instance) = &*instance {
                    if node_ids.contains(&component_instance.id().node_id) {
                        instance.mark_lost();
                        changed = true;
                    }
                }
            }
        }
        changed
    }
}
