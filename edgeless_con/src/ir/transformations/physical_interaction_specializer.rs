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

impl super::StatefulTransformation<PhysicalInteractionSpecializerState> for PhysicalInteractionSpecializer {
    #[tracing::instrument(name = "physical_interaction_specializer", skip_all)]
    fn apply(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        global_state: &PhysicalInteractionSpecializerState,
    ) {
        if workflow.original_request.annotations.contains_key("DISABLE_MULTICAST") {
            return;
        }

        let mut reg = global_state.dialect_registry.blocking_lock();

        let Ok(interactions) = collect_physical_interactions(workflow, &mut reg) else {
            tracing::warn!("Failure Collecting Interactions");
            return;
        };

        let mapped_interactions = interactions
            .into_iter()
            .flat_map(|i| {
                try_map_interaction(&i, nodes, &mut reg).unwrap_or_else(|e| {
                    tracing::debug!("Failed to map interaction: {e}; Falling back to original mapping");
                    vec![i]
                })
            })
            .collect();

        for i in &mapped_interactions {
            let link_config = reg.link_config(i, nodes);

            match link_config {
                interaction::LinkConfigurationResult::Ok(workflow_link) => {
                    workflow.links.entry(workflow_link.id.clone()).or_insert(workflow_link);
                }
                interaction::LinkConfigurationResult::NoConfig => {
                    tracing::debug!("No Link Configuration Required");
                }
                interaction::LinkConfigurationResult::Err(link_configuration_error) => {
                    tracing::warn!("Link Configuration Error {link_configuration_error}");
                }
            }
        }

        if let Err(e) = distribute_physical_interactions(mapped_interactions, workflow, &mut reg) {
            tracing::warn!("Failure distributing physical interactions: {e}");
        }
    }
}

fn try_map_interaction(
    src: &interaction::InteractionMapping,
    nodes: &crate::ir::Nodes,
    reg: &mut interaction::dialect::DialectRegistry,
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

fn collect_physical_interactions(
    workflow: &mut crate::ir::workflow::ActiveWorkflow,
    dialect_registry: &mut crate::ir::interaction::dialect::DialectRegistry,
) -> Result<Vec<crate::ir::interaction::InteractionMapping>, crate::ir::interaction::InteractionError> {
    let mut port_collector = std::collections::BTreeMap::<
        crate::ir::interaction::dialect::DialectDescriptor,
        (
            Vec<(crate::ir::interaction::PhysicalPortId, crate::ir::interaction::SourcePortMapping)>,
            Vec<(crate::ir::interaction::PhysicalPortId, crate::ir::interaction::DestiantionPortMapping)>,
        ),
    >::new();

    for (_c_id, c) in workflow.components() {
        let mut current = c.borrow_mut();
        let (_logical_ports, physical_instances) = current.split_view();
        for i in &physical_instances {
            if let Some(i) = i.borrow_mut().try_unpack_active_mut() {
                let cloned_id = i.id().clone();
                let ports = i.physical_ports();

                for (out_id, out) in std::mem::take(&mut ports.physical_output_mapping) {
                    port_collector.entry(out.dialect_type.clone()).or_default().0.push((
                        crate::ir::interaction::PhysicalPortId {
                            instance: cloned_id.clone(),
                            port: out_id,
                        },
                        out,
                    ));
                }

                for (input_id, input) in std::mem::take(&mut ports.physical_input_mapping) {
                    port_collector.entry(input.dialect_type.clone()).or_default().1.push((
                        crate::ir::interaction::PhysicalPortId {
                            instance: cloned_id.clone(),
                            port: input_id,
                        },
                        input,
                    ));
                }
            }
        }
    }

    Ok(port_collector
        .into_iter()
        .map(|(dialect, (source_ports, destination_ports))| dialect_registry.physical_ports_to_interaction(&dialect, source_ports, destination_ports))
        .collect::<Result<Vec<_>, crate::ir::interaction::InteractionError>>()?
        .into_iter()
        .flatten()
        .collect())
}

fn distribute_physical_interactions(
    mapped_interactions: Vec<crate::ir::interaction::InteractionMapping>,
    workflow: &mut crate::ir::workflow::ActiveWorkflow,
    dialect_registry: &mut crate::ir::interaction::dialect::DialectRegistry,
) -> Result<(), crate::ir::interaction::InteractionError> {
    let mut replacement_srcs = std::collections::BTreeMap::<
        edgeless_api::function_instance::InstanceId,
        std::collections::BTreeMap<edgeless_api::function_instance::PortId, interaction::SourcePortMapping>,
    >::new();
    let mut replacement_dests = std::collections::BTreeMap::<
        edgeless_api::function_instance::InstanceId,
        std::collections::BTreeMap<edgeless_api::function_instance::PortId, interaction::DestiantionPortMapping>,
    >::new();

    for i in mapped_interactions {
        let (s, d) = dialect_registry.physical_interaction_to_ports(i)?;
        for (port_id, source_spec) in s {
            replacement_srcs
                .entry(port_id.instance.clone())
                .or_default()
                .insert(port_id.port.clone(), source_spec);
        }
        for (port_id, dest_spec) in d {
            replacement_dests
                .entry(port_id.instance.clone())
                .or_default()
                .insert(port_id.port.clone(), dest_spec);
        }
    }

    for (_c_id, c) in workflow.components() {
        let mut current = c.borrow_mut();
        let (_logical_ports, physical_instances) = current.split_view();
        for i in &physical_instances {
            if let Some(i) = i.borrow_mut().try_unpack_active_mut() {
                let cloned_id = i.id();
                let ports = i.physical_ports();

                let inputs = replacement_dests.remove(&cloned_id).unwrap_or_default();
                let outputs = replacement_srcs.remove(&cloned_id).unwrap_or_default();

                for (input_port, port_mapping) in inputs {
                    ports.physical_input_mapping.insert(input_port, port_mapping);
                }

                for (output_port, port_mapping) in outputs {
                    ports.physical_output_mapping.insert(output_port, port_mapping);
                }
            }
        }
    }

    Ok(())
}
