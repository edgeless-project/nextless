// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct PhysicalInteractionNormalizer {}

impl PhysicalInteractionNormalizer {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatelessPhysicalTransformation for PhysicalInteractionNormalizer {
    #[tracing::instrument(name = "physical_interaction_normalizer", skip_all)]
    fn apply(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
    ) -> Vec<super::PhysicalChange> {
        let mut dialect_registry = crate::ir::interaction::dialect::DialectRegistry::only_physical_overlay();
        let Ok(interactions) = super::physical_interaction_specializer::collect_physical_interactions(workflow, &mut dialect_registry) else {
            return Vec::new();
        };
        super::physical_interaction_specializer::distribute_physical_interactions(interactions, workflow, &mut dialect_registry)
            .map_err(|e| {
                tracing::warn!("Failure distributing physical interactions: {e}");
            })
            .unwrap_or_default()
    }
}
