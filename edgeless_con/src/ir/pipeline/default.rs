// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use super::super::transformations::placement::strategy::PlacementStrategy;

pub struct DefaultTransformationPipeline<P: PlacementStrategy> {
    logical_pipeline: super::default_logical::DefaultLogicalPipeline,
    orchestration: super::default_orchestration::DefaultOrchestrationPipeline<P>,
    physical_pipeline: super::default_physical::DefaultPhysicalPipeline,
}

pub struct DefaultTransformationPipelineState<PS: Sync + Send> {
    pub placement_strategy_state: PS,
    pub physical_pipeline_state: super::default_physical::PhysicalPipelineState,
}

impl<P: PlacementStrategy> DefaultTransformationPipeline<P> {
    pub fn new_default(placement_strategy: P) -> Self {
        Self {
            logical_pipeline: super::default_logical::DefaultLogicalPipeline::new(),
            orchestration: super::default_orchestration::DefaultOrchestrationPipeline::new(placement_strategy),
            physical_pipeline: super::default_physical::DefaultPhysicalPipeline::new(),
        }
    }
}

impl<P: PlacementStrategy> super::TransformationPipeline<DefaultTransformationPipelineState<P::GlobalState>> for DefaultTransformationPipeline<P> {
    fn apply_all(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &DefaultTransformationPipelineState<P::GlobalState>,
    ) {
        self.logical_pipeline.apply_all(workflow, nodes, peer_clusters, &());
        self.orchestration
            .apply_all(workflow, nodes, peer_clusters, &global_state.placement_strategy_state);
        self.physical_pipeline
            .apply_all(workflow, nodes, peer_clusters, &global_state.physical_pipeline_state);
    }

    fn apply_dynamic(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &DefaultTransformationPipelineState<P::GlobalState>,
    ) {
        self.orchestration
            .apply_all(workflow, nodes, peer_clusters, &global_state.placement_strategy_state);
        self.physical_pipeline
            .apply_all(workflow, nodes, peer_clusters, &global_state.physical_pipeline_state);
    }
}
