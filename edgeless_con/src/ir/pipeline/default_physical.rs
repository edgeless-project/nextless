// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::ir::transformations::{StatefulTransformation, StatelessTransformation};

pub struct DefaultPhysicalPipeline {
    physical_connection_mapper: crate::ir::transformations::physical_mapper::PhysicalConnectionMapper,
    pipe_generator: crate::ir::transformations::pipe_generator::PipeGenerator,
    compiler: crate::ir::transformations::compiler::Compiler,
}

pub struct PhysicalPipelineState {
    pub pipe_generator_state: crate::ir::transformations::pipe_generator::PipeGeneratorState,
    pub compiler_state: crate::ir::transformations::compiler::CompilerStore,
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

impl crate::ir::pipeline::TransformationPipeline<PhysicalPipelineState> for DefaultPhysicalPipeline {
    fn apply_all(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &PhysicalPipelineState,
    ) {
        self.physical_connection_mapper.apply(workflow, nodes, peer_clusters);
        self.pipe_generator
            .apply(workflow, nodes, peer_clusters, &global_state.pipe_generator_state);
        self.compiler.apply(workflow, nodes, peer_clusters, &global_state.compiler_state);
    }

    fn apply_dynamic(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &PhysicalPipelineState,
    ) {
        self.apply_all(workflow, nodes, peer_clusters, global_state);
    }
}
