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
    pub interaction_dialect_registry: std::sync::Arc<tokio::sync::Mutex<crate::ir::interaction::dialect::DialectRegistry>>,
    pub image_cache: crate::ir::support::image_cache::ImageCache,
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
        tracing::info_span!("logical_pipeline").in_scope(|| {
            self.logical_pipeline.apply_all(
                workflow,
                nodes,
                peer_clusters,
                &super::default_logical::LogicalPipelineState {
                    logical_interaction_normalizer_state:
                        &crate::ir::transformations::logical_interaction_normalizer::LogicalInteractionNormalizerState {
                            dialect_registry: global_state.interaction_dialect_registry.clone(),
                        },
                },
            )
        });
        tracing::info_span!("physical_pipeline").in_scope(|| {
            self.orchestration.apply_all(
                workflow,
                nodes,
                peer_clusters,
                &crate::ir::transformations::placement::PlacementState::<P> {
                    strategy_state: &global_state.placement_strategy_state,
                    image_chache: &global_state.image_cache,
                },
            );
            self.physical_pipeline.apply_all(
                workflow,
                nodes,
                peer_clusters,
                &super::default_physical::PhysicalPipelineState {
                    pipe_generator_state: &crate::ir::transformations::physical_interaction_specializer::PhysicalInteractionSpecializerState::new(
                        global_state.interaction_dialect_registry.clone(),
                    ),
                    compiler_state: &global_state.image_cache,
                },
            );
        });
    }

    fn apply_dynamic(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &DefaultTransformationPipelineState<P::GlobalState>,
    ) {
        tracing::info_span!("physical_pipeline").in_scope(|| {
            self.orchestration.apply_all(
                workflow,
                nodes,
                peer_clusters,
                &crate::ir::transformations::placement::PlacementState::<P> {
                    strategy_state: &global_state.placement_strategy_state,
                    image_chache: &global_state.image_cache,
                },
            );
            self.physical_pipeline.apply_all(
                workflow,
                nodes,
                peer_clusters,
                &super::default_physical::PhysicalPipelineState {
                    pipe_generator_state: &crate::ir::transformations::physical_interaction_specializer::PhysicalInteractionSpecializerState::new(
                        global_state.interaction_dialect_registry.clone(),
                    ),
                    compiler_state: &global_state.image_cache,
                },
            );
        });
    }
}
