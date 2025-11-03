// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::ir::transformations::{StatefulTransformation, StatelessTransformation};

pub struct DefaultLogicalPipeline {
    topic_converter: crate::ir::transformations::logical_interaction_normalizer::LogicalInteractionNormalizer,
    input_linker: crate::ir::transformations::input_linker::InputLinker,
    workflow_splitter: crate::ir::transformations::workflow_spitter::WorkflowSplitter,
    dead_component_removal: crate::ir::transformations::dead_component_removal::DeadComponentRemoval,
}

pub struct LogicalPipelineState<'a> {
    pub logical_interaction_normalizer_state: &'a crate::ir::transformations::logical_interaction_normalizer::LogicalInteractionNormalizerState,
}

impl DefaultLogicalPipeline {
    pub fn new() -> Self {
        DefaultLogicalPipeline {
            topic_converter: crate::ir::transformations::logical_interaction_normalizer::LogicalInteractionNormalizer::new(),
            input_linker: crate::ir::transformations::input_linker::InputLinker::new(),
            workflow_splitter: crate::ir::transformations::workflow_spitter::WorkflowSplitter::new(),
            dead_component_removal: crate::ir::transformations::dead_component_removal::DeadComponentRemoval::new(),
        }
    }
}

impl<'a> super::TransformationPipeline<LogicalPipelineState<'a>> for DefaultLogicalPipeline {
    fn apply_all(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &LogicalPipelineState,
    ) {
        self.topic_converter
            .apply(workflow, nodes, peer_clusters, &global_state.logical_interaction_normalizer_state);
        self.input_linker.apply(workflow, nodes, peer_clusters);
        self.workflow_splitter.apply(workflow, nodes, peer_clusters);
        self.dead_component_removal.apply(workflow, nodes, peer_clusters);
    }

    fn apply_dynamic(
        &mut self,
        _workflow: &mut crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        _global_state: &LogicalPipelineState,
    ) {
    }
}
