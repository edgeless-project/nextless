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
        let node_removal_changes = self.remove_nodes(removed_node_ids);
        if node_removal_changes.is_empty() {
            return vec![];
        }

        self.wf.apply_physical_changes(node_removal_changes);
        self.pipeline.apply_dynamic(&mut self.wf, nodes, peer_clusters, global_state);
        self.materialize()
    }

    #[tracing::instrument(name = "engine_patch_external_links", skip_all, fields(workflow_id = self.wf.id.to_string()))]
    pub fn patch_external_links(
        &mut self,
        _update: edgeless_api::common::PatchRequest,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        _global_state: &super::pipeline::default::DefaultTransformationPipelineState<P::GlobalState>,
    ) -> Vec<super::RequiredChange> {
        todo!("Proxy handling not implemented yet")
        // {
        //     let mut prx = self.wf.proxy.borrow_mut();
        //     prx.external_ports.external_input_mapping = crate::ir::physical_model::parse_api_input_mapping(update.input_mapping);
        //     prx.external_ports.external_output_mapping = crate::ir::physical_model::parse_api_output_mapping(update.output_mapping);
        // }
        // self.pipeline.apply_dynamic(&mut self.wf, nodes, peer_clusters, global_state);
        // self.materialize()
    }

    #[allow(unused)]
    pub fn peer_cluster_removal(&self, _removed_cluster_ids: edgeless_api::function_instance::NodeId) -> Vec<super::RequiredChange> {
        Vec::new()
    }

    pub fn stop(
        &mut self,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &super::pipeline::default::DefaultTransformationPipelineState<P::GlobalState>,
    ) -> Vec<super::RequiredChange> {
        self.pipeline.apply_stop(&mut self.wf, nodes, peer_clusters, global_state);
        self.materialize()
    }

    #[tracing::instrument(name = "calculate_required_changes", skip_all)]
    fn materialize(&mut self) -> Vec<super::RequiredChange> {
        let mut changes = Vec::new();
        let mut model_changes = Vec::new();

        for (link_id, link) in &self.wf.links {
            let mut changed = false;
            let mut cloned_link = link.clone();
            if !link.materialized {
                changes.push(super::RequiredChange::InstantiateLinkControlPlane {
                    link_id: link_id.clone(),
                    class: link.class.clone(),
                });
                cloned_link.materialized = true;
                changed = true;
            }

            for (node, link_provider_id, node_config, node_materialized) in &mut cloned_link.nodes {
                if !*node_materialized {
                    changes.push(super::RequiredChange::CreateLinkOnNode {
                        node_id: *node,
                        provider_id: link_provider_id.clone(),
                        link_id: link_id.clone(),
                        config: node_config.clone(),
                    });
                    *node_materialized = true;
                    changed = true;
                }
            }

            if changed {
                model_changes.push(crate::ir::transformations::PhysicalChange::Link(
                    crate::ir::transformations::PhysicalLinkChange {
                        link_id: link_id.clone(),
                        action: crate::ir::transformations::PhysicalLinkChangeAction::Update(cloned_link),
                    },
                ));
            }
        }

        for (_c_name, _logical_component, component_instances) in self.wf.components_with_instances() {
            for i in component_instances {
                match i.component {
                    super::PhysicalComponentState::Planned(planned_instance) => {
                        let (instance_model_changes, material_changes) = planned_instance.materialize(&self.telemetry_provider);
                        changes.extend(material_changes);
                        model_changes.extend(instance_model_changes);
                    }
                    super::PhysicalComponentState::Materialized(maybe_dirty_instance) => {
                        let (instance_model_changes, material_changes) = maybe_dirty_instance.materialize(&self.telemetry_provider);
                        changes.extend(material_changes);
                        model_changes.extend(instance_model_changes);
                    }
                    super::PhysicalComponentState::StopPlanned { old, .. } => {
                        changes.extend(old.stop());
                        model_changes.extend(i.mark_stopped());
                    }
                    _ => {
                        // NOOP
                    }
                }
            }
        }
        self.wf.apply_physical_changes(model_changes);

        changes
    }

    fn remove_nodes(
        &self,
        node_ids: &std::collections::HashSet<edgeless_api::function_instance::NodeId>,
    ) -> Vec<crate::ir::transformations::PhysicalChange> {
        let mut required_changes = Vec::new();
        for (_, _logical_component, component_instances) in self.wf.components_with_instances() {
            for instance in component_instances {
                if let super::PhysicalComponentState::Materialized(component_instance) = instance.component {
                    if node_ids.contains(&component_instance.id().node_id) {
                        required_changes.extend(instance.mark_lost());
                    }
                }
            }
        }
        required_changes
    }
}
