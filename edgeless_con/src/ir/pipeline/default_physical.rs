// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::ir::transformations::{StatefulTransformation, StatelessTransformation};

pub struct DefaultPhysicalPipeline {
    physical_connection_mapper: crate::ir::transformations::physical_mapper::PhysicalConnectionMapper,
    pipe_generator: crate::ir::transformations::pipe_generator::PipeGenerator,
    compiler: crate::ir::transformations::compiler::Compiler,
}

impl DefaultPhysicalPipeline {
    pub fn new() -> Self {
        DefaultPhysicalPipeline {
            physical_connection_mapper: crate::ir::transformations::physical_mapper::PhysicalConnectionMapper::new(),
            pipe_generator: crate::ir::transformations::pipe_generator::PipeGenerator::new(),
            compiler: crate::ir::transformations::compiler::Compiler::new(),
        }
    }
}

impl crate::ir::pipeline::TransformationPipeline<crate::ir::transformations::pipe_generator::PipeGeneratorState> for DefaultPhysicalPipeline {
    fn apply_all(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &mut crate::ir::transformations::pipe_generator::PipeGeneratorState,
    ) {
        self.physical_connection_mapper.apply(workflow, nodes, peer_clusters);
        self.pipe_generator.apply(workflow, nodes, peer_clusters, global_state);
        self.compiler.apply(workflow, nodes, peer_clusters);
    }

    fn apply_dynamic(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &mut crate::ir::transformations::pipe_generator::PipeGeneratorState,
    ) {
        self.apply_all(workflow, nodes, peer_clusters, global_state);
    }
}
