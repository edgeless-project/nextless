// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::ir::transformations::{StatefulPhysicalTransformation, StatelessPhysicalTransformation};

pub struct DefaultPhysicalPipeline {
    physical_connection_mapper: crate::ir::transformations::physical_mapper::PhysicalConnectionMapper,
    physical_interaction_normalizer: crate::ir::transformations::physical_interaction_normalizer::PhysicalInteractionNormalizer,
    dead_instance_removal: crate::ir::transformations::dead_instance_removal::DeadInstanceRemoval,
    pipe_generator: crate::ir::transformations::physical_interaction_specializer::PhysicalInteractionSpecializer,
    compiler: crate::ir::transformations::compiler::Compiler,
}

pub struct PhysicalPipelineState<'a> {
    pub pipe_generator_state: &'a crate::ir::transformations::physical_interaction_specializer::PhysicalInteractionSpecializerState,
    pub compiler_state: &'a crate::ir::support::image_cache::ImageCache,
}

impl DefaultPhysicalPipeline {
    pub fn new() -> Self {
        DefaultPhysicalPipeline {
            physical_connection_mapper: crate::ir::transformations::physical_mapper::PhysicalConnectionMapper::new(),
            physical_interaction_normalizer: crate::ir::transformations::physical_interaction_normalizer::PhysicalInteractionNormalizer::new(),
            dead_instance_removal: crate::ir::transformations::dead_instance_removal::DeadInstanceRemoval::new(),
            pipe_generator: crate::ir::transformations::physical_interaction_specializer::PhysicalInteractionSpecializer::new(),
            compiler: crate::ir::transformations::compiler::Compiler::new(),
        }
    }
}

impl<'a> crate::ir::pipeline::TransformationPipeline<PhysicalPipelineState<'a>> for DefaultPhysicalPipeline {
    fn apply_all(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &PhysicalPipelineState,
    ) {
        self.apply_dynamic(workflow, nodes, peer_clusters, global_state);
    }

    fn apply_dynamic(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &PhysicalPipelineState,
    ) {
        let changes = self.physical_connection_mapper.apply(workflow, nodes, peer_clusters);
        if changes.len() > 0 {
            tracing::debug!("Connection Mapper: {changes:?}");
        }
        workflow.apply_physical_changes(changes);
        let changes = self.physical_interaction_normalizer.apply(workflow, nodes, peer_clusters);
        if changes.len() > 0 {
            tracing::debug!("Physical Interaction Normalizer: {changes:?}");
        }
        workflow.apply_physical_changes(changes);
        let changes = self.dead_instance_removal.apply(workflow, nodes, peer_clusters);
        if changes.len() > 0 {
            tracing::debug!("Dead Instance Removal: {changes:?}");
        }
        workflow.apply_physical_changes(changes);
        let changes = self
            .pipe_generator
            .apply(workflow, nodes, peer_clusters, global_state.pipe_generator_state);
        if changes.len() > 0 {
            tracing::debug!("Interaction Specializer: {changes:?}");
        }
        workflow.apply_physical_changes(changes);
        let changes = self.compiler.apply(workflow, nodes, peer_clusters, global_state.compiler_state);
        if changes.len() > 0 {
            tracing::debug!("Compiler: {changes:?}");
        }
        workflow.apply_physical_changes(changes);
    }

    fn apply_stop(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &PhysicalPipelineState,
    ) {
        let changes = self
            .pipe_generator
            .apply_stop(workflow, nodes, peer_clusters, global_state.pipe_generator_state);
        if changes.len() > 0 {
            tracing::debug!("Interaction Specializer Stop: {changes:?}");
        }
        workflow.apply_physical_changes(changes);
        let changes = self.compiler.apply_stop(workflow, nodes, peer_clusters, global_state.compiler_state);
        if changes.len() > 0 {
            tracing::debug!("Compiler Stop: {changes:?}");
        }
        workflow.apply_physical_changes(changes);
    }
}
