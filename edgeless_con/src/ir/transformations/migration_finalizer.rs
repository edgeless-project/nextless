// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct MigrationFinalizer {}

impl MigrationFinalizer {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatelessPhysicalTransformation for MigrationFinalizer {
    #[tracing::instrument(name = "migration_finalizer", skip_all)]
    fn apply(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
    ) -> Vec<super::PhysicalChange> {
        let mut required_changes = Vec::new();

        for (_, _component, component_instances) in workflow.components_with_instances() {
            'each_instance: for instance in component_instances.clone() {
                if let crate::ir::PhysicalComponentState::MigratingAway { new, .. } = &*instance.component {
                    tracing::debug!("There is a component instance under migration.");
                    for other_instance in component_instances.clone() {
                        if let crate::ir::PhysicalComponentState::Materialized(other_component_instance) = &*other_instance.component {
                            if other_component_instance.id() == *new {
                                tracing::debug!("Found replacement for migrating component.");
                                required_changes.extend(instance.plan_stop());
                                continue 'each_instance;
                            }
                        }
                    }
                }
            }
        }

        required_changes
    }
}
