// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct MigrationFinalizer {}

impl MigrationFinalizer {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatelessTransformation for MigrationFinalizer {
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, _nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        for (_, component) in workflow.components() {
            let component = component.borrow_mut();
            'each_instance: for instance in component.instances() {
                let mut instance = instance.borrow_mut();
                if let crate::ir::PhysicalComponentState::MigratingAway { new, .. } = &*instance {
                    log::info!("There is a Component Instance Under Migration");
                    for other_instance in component.instances() {
                        // This will fail for 'instance', which does not matter here
                        if let Ok(other_instance) = other_instance.try_borrow() {
                            if let crate::ir::PhysicalComponentState::Materialized(other_component_instance) = &*other_instance {
                                if other_component_instance.id() == *new {
                                    log::info!("Found Replacepment");
                                    instance.plan_stop();
                                    continue 'each_instance;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
