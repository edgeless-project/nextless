// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod compiler;
pub mod dead_component_removal;
pub mod input_linker;
pub mod physical_mapper;
pub mod pipe_generator;
pub mod placement;
pub mod topic_converter;
pub mod workflow_spitter;

trait Transformation: Send + Sync {
    fn apply(&mut self, workflow: &mut super::workflow::ActiveWorkflow);
}

pub struct TransformationPipeline {
    logical_pipeline: Vec<Box<dyn Transformation>>,
    placement: Vec<Box<dyn Transformation>>,
    physical_pipeline: Vec<Box<dyn Transformation>>,
}

impl TransformationPipeline {
    pub fn new_default(
        orchestration_logic: std::sync::Arc<tokio::sync::Mutex<crate::orchestration_logic::OrchestrationLogic>>,
        nodes: std::sync::Arc<
            tokio::sync::Mutex<std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>>,
        >,
        peer_clusters: std::sync::Arc<
            tokio::sync::Mutex<std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::PeerCluster>>,
        >,
        link_controllers: std::sync::Arc<
            tokio::sync::Mutex<std::collections::HashMap<edgeless_api::link::LinkType, Box<dyn edgeless_api::link::LinkController>>>,
        >,
    ) -> Self {
        Self {
            logical_pipeline: vec![
                Box::new(topic_converter::TopicConverter::new()),
                Box::new(input_linker::InputLinker::new()),
                Box::new(workflow_spitter::WorkflowSplitter::new()),
                Box::new(dead_component_removal::DeadComponentRemoval::new()),
            ],
            placement: vec![
                Box::new(placement::DefaultPlacement::new(orchestration_logic, nodes.clone(), peer_clusters)),
                Box::new(physical_mapper::PhysicalConnectionMapper::new()),
            ],
            physical_pipeline: vec![
                Box::new(pipe_generator::PipeGenerator::new(nodes.clone(), link_controllers.clone())),
                Box::new(compiler::Compiler::new()),
            ],
        }
    }

    pub fn apply_all(&mut self, workflow: &mut super::workflow::ActiveWorkflow) {
        for t in &mut self.logical_pipeline {
            t.apply(workflow);
        }
        for t in &mut self.placement {
            t.apply(workflow);
        }
        for t in &mut self.physical_pipeline {
            t.apply(workflow);
        }
    }
}
