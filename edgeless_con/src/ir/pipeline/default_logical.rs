// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::ir::transformations::{StatefulLogicalTransformation, StatelessLogicalTransformation};

pub struct DefaultLogicalPipeline {
    topic_converter: crate::ir::transformations::logical_interaction_normalizer::LogicalInteractionNormalizer,
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
            workflow_splitter: crate::ir::transformations::workflow_spitter::WorkflowSplitter::new(),
            dead_component_removal: crate::ir::transformations::dead_component_removal::DeadComponentRemoval::new(),
        }
    }
}

impl<'a> super::TransformationPipeline<LogicalPipelineState<'a>> for DefaultLogicalPipeline {
    fn apply_all(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        global_state: &LogicalPipelineState,
    ) {
        let changes = self.topic_converter.apply(workflow, &global_state.logical_interaction_normalizer_state);
        workflow.apply_logical_changes(changes);
        let changes = self.workflow_splitter.apply(workflow);
        workflow.apply_logical_changes(changes);
        let changes = self.dead_component_removal.apply(workflow);
        workflow.apply_logical_changes(changes);
    }

    fn apply_dynamic(
        &mut self,
        _workflow: &mut crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        _global_state: &LogicalPipelineState,
    ) {
    }

    fn apply_stop(
        &mut self,
        _workflow: &mut crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        _global_state: &LogicalPipelineState,
    ) {
        // There currently is no stateful logical component.
    }
}
