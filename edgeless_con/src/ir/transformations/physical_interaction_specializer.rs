// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::super::*;

pub struct PhysicalInteractionSpecializer {}

impl PhysicalInteractionSpecializer {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct PhysicalInteractionSpecializerState {
    dialect_registry: std::sync::Arc<tokio::sync::Mutex<crate::ir::interaction::dialect::DialectRegistry>>,
}

impl PhysicalInteractionSpecializerState {
    pub fn new(dialect_registry: std::sync::Arc<tokio::sync::Mutex<crate::ir::interaction::dialect::DialectRegistry>>) -> Self {
        Self { dialect_registry }
    }
}

impl super::StatefulPhysicalTransformation<PhysicalInteractionSpecializerState> for PhysicalInteractionSpecializer {
    #[tracing::instrument(name = "physical_interaction_specializer", skip_all)]
    fn apply(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        global_state: &PhysicalInteractionSpecializerState,
    ) -> Vec<super::PhysicalChange> {
        let mut required_changes = Vec::new();

        let mut reg = global_state.dialect_registry.blocking_lock();

        let Ok(interactions) = support::port_utils::collect_physical_interactions(workflow, &mut reg) else {
            tracing::warn!("Failure collecting interactions");
            return vec![];
        };

        let mapped_interactions = interactions
            .into_iter()
            .flat_map(|i| {
                try_map_interaction(&i, nodes, &mut reg, workflow.feature_flags.disable_ip_multicast_dialect).unwrap_or_else(|e| {
                    tracing::debug!("Failed to map interaction: {e}; Falling back to original mapping");
                    vec![i]
                })
            })
            .collect();

        for i in &mapped_interactions {
            let link_config = reg.link_config(i, nodes);

            match link_config {
                interaction::LinkConfigurationResult::Ok(workflow_link) => {
                    if let Some(existing) = workflow.links.get(&workflow_link.id) {
                        if *existing != workflow_link {
                            required_changes.push(super::PhysicalChange::Link(super::PhysicalLinkChange {
                                link_id: workflow_link.id.clone(),
                                action: super::PhysicalLinkChangeAction::Update(workflow_link),
                            }));
                        }
                    } else {
                        required_changes.push(super::PhysicalChange::Link(super::PhysicalLinkChange {
                            link_id: workflow_link.id.clone(),
                            action: super::PhysicalLinkChangeAction::Insert(workflow_link),
                        }));
                    }
                }
                interaction::LinkConfigurationResult::NoConfig => {
                    tracing::debug!("No Link Configuration Required");
                }
                interaction::LinkConfigurationResult::Err(link_configuration_error) => {
                    tracing::warn!("Link configuration error: {link_configuration_error}");
                }
            }
        }

        if let Err(e) = support::port_utils::distribute_physical_interactions(mapped_interactions, workflow, &mut reg) {
            tracing::warn!("Failure distributing physical interactions: {e}");
        }

        return required_changes;
    }

    fn apply_stop(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        _global_state: &PhysicalInteractionSpecializerState,
    ) -> Vec<transformations::PhysicalChange> {
        let mut required_changes = Vec::new();

        for (link_id, _link) in &workflow.links {
            // This would probably actually require tracking the links' state similar to how we track the component instances.
            tracing::warn!("Links are currently not removed from the nodes!");
            // TODO: This should be an update
            required_changes.push(transformations::PhysicalChange::Link(transformations::PhysicalLinkChange {
                link_id: link_id.clone(),
                action: transformations::PhysicalLinkChangeAction::Delete,
            }));
        }

        required_changes
    }
}

fn try_map_interaction(
    src: &interaction::InteractionMapping,
    nodes: &crate::ir::Nodes,
    reg: &mut interaction::dialect::DialectRegistry,
    disable_ip_multicast_dialect: bool,
) -> Result<Vec<interaction::InteractionMapping>, crate::ir::interaction::InteractionError> {
    let node_ids = src
        .mapping
        .as_physical()
        .ok_or(interaction::InteractionError::UnexpectedDialect)?
        .relevant_nodes();

    let mut supported_dialects = std::collections::BTreeMap::new();

    for node_id in node_ids {
        if let Some(node) = nodes.get(&node_id) {
            let node_dialects: std::collections::BTreeMap<_, _> = node
                .available_interaction_dialects()
                .into_iter()
                .filter(|d| d.base_type != interaction::dialect::ip_multicast::ID || !disable_ip_multicast_dialect)
                .map(|d| (d.base_type, d.constraints))
                .collect();

            if supported_dialects.is_empty() {
                for (base_type, constraints) in &node_dialects {
                    supported_dialects.insert(base_type.clone(), constraints.clone());
                }
            } else {
                supported_dialects.retain(|base_type, constraints| {
                    node_dialects
                        .get(base_type)
                        .is_some_and(|existing_constraints| existing_constraints == constraints)
                });
            }
        } else {
            return Err(crate::ir::interaction::InteractionError::TranslationError(anyhow::anyhow!(
                "Could not find node corresponding to instance while performing interaction mapping."
            )));
        }
    }

    let mut translation_plans: Vec<_> = supported_dialects
        .into_iter()
        .filter_map(|(base, constraints)| {
            let d = &crate::ir::interaction::dialect::DialectDescriptor {
                base_type: base,
                constraints: constraints,
            };

            reg.plan_translation(&src, &d)
                .map_err(|e| {
                    tracing::debug!("Interaction translation plan failed: {:?} -> {:?}", src.dialect_type.base_type, base);
                    e
                })
                .ok()
        })
        .collect();

    translation_plans.sort_by(|a, b| b.1.cmp(&a.1));

    for (target, _score) in translation_plans {
        let translation = reg.try_translate(&src, &target);

        if let Ok(translation) = translation {
            return Ok(translation);
        } else {
            tracing::debug!(
                "Interaction translation failed: {:?} -> {:?}",
                src.dialect_type.base_type,
                target.base_type
            );
        }
    }

    return Err(crate::ir::interaction::InteractionError::UnsupportedTranslation(
        src.dialect_type.clone(),
        // TODO might need to add an error case for this.
        src.dialect_type.clone(),
    ));
}
