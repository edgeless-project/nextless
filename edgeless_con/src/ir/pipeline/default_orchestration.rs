// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use super::super::transformations::placement::strategy::PlacementStrategy;
use crate::ir::transformations::{StatefulTransformation, StatelessTransformation};

pub struct DefaultOrchestrationPipeline<P: PlacementStrategy> {
    scaler: super::super::transformations::scaler::Scaler,
    placement: super::super::transformations::placement::DefaultPlacement<P>,
}

impl<P: PlacementStrategy> DefaultOrchestrationPipeline<P> {
    pub fn new(placement_strategy: P) -> Self {
        DefaultOrchestrationPipeline {
            scaler: super::super::transformations::scaler::Scaler::new(),
            placement: crate::ir::transformations::placement::DefaultPlacement::new(placement_strategy),
        }
    }
}

impl<P: PlacementStrategy> super::TransformationPipeline<P::GlobalState> for DefaultOrchestrationPipeline<P> {
    fn apply_all(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &mut P::GlobalState,
    ) {
        self.apply_dynamic(workflow, nodes, peer_clusters, global_state);
    }

    fn apply_dynamic(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &mut P::GlobalState,
    ) {
        self.scaler.apply(workflow, nodes, peer_clusters);
        self.placement.apply(workflow, nodes, peer_clusters, global_state);
    }
}
