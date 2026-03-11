// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use super::super::transformations::placement::strategy::PlacementStrategy;
use crate::ir::transformations::{StatefulPhysicalTransformation, StatelessPhysicalTransformation};

pub struct DefaultOrchestrationPipeline<P: PlacementStrategy> {
    scaler: super::super::transformations::scaler::Scaler,
    colocation_optimizer: super::super::transformations::colocation_optimizer::ColocationOptimizer,
    migration_finalizer: super::super::transformations::migration_finalizer::MigrationFinalizer,
    placement: super::super::transformations::placement::DefaultPlacement<P>,
}

impl<P: PlacementStrategy> DefaultOrchestrationPipeline<P> {
    pub fn new(placement_strategy: P) -> Self {
        DefaultOrchestrationPipeline {
            scaler: super::super::transformations::scaler::Scaler::new(),
            colocation_optimizer: super::super::transformations::colocation_optimizer::ColocationOptimizer::new(),
            migration_finalizer: super::super::transformations::migration_finalizer::MigrationFinalizer::new(),
            placement: crate::ir::transformations::placement::DefaultPlacement::new(placement_strategy),
        }
    }
}

impl<'a, P: PlacementStrategy> super::TransformationPipeline<crate::ir::transformations::placement::PlacementState<'a, P>>
    for DefaultOrchestrationPipeline<P>
{
    fn apply_all(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &crate::ir::transformations::placement::PlacementState<P>,
    ) {
        self.apply_dynamic(workflow, nodes, peer_clusters, global_state);
    }

    fn apply_dynamic(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &crate::ir::transformations::placement::PlacementState<P>,
    ) {
        let changes = self.scaler.apply(workflow, nodes, peer_clusters);
        if changes.len() > 0 {
            tracing::debug!("Scaler: {changes:?}");
        }
        workflow.apply_physical_changes(changes);
        let changes = self.colocation_optimizer.apply(workflow, nodes, peer_clusters);
        if changes.len() > 0 {
            tracing::debug!("Colocation Optimizer: {changes:?}");
        }
        workflow.apply_physical_changes(changes);
        let changes = self.migration_finalizer.apply(workflow, nodes, peer_clusters);
        if changes.len() > 0 {
            tracing::debug!("Migration Finalizer: {changes:?}");
        }
        workflow.apply_physical_changes(changes);
        let changes = self.placement.apply(workflow, nodes, peer_clusters, global_state);
        if changes.len() > 0 {
            tracing::debug!("Placement: {changes:?}");
        }
        workflow.apply_physical_changes(changes);
    }

    fn apply_stop(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &crate::ir::transformations::placement::PlacementState<P>,
    ) {
        let changes = self.placement.apply_stop(workflow, nodes, peer_clusters, global_state);
        if changes.len() > 0 {
            tracing::debug!("Placement Stop: {changes:?}");
        }
        workflow.apply_physical_changes(changes);
    }
}
