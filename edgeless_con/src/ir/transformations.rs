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

pub trait StatelessTransformation: Send + Sync {
    fn apply(&mut self, workflow: &mut super::workflow::ActiveWorkflow, nodes: &crate::ir::Nodes, peer_clusters: &crate::ir::Clusters);
}

pub trait StatefulTransformation<G>: Send + Sync {
    fn apply(
        &mut self,
        workflow: &mut super::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &mut G,
    );
}
