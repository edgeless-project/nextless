// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::super::*;

pub struct LogicalInteractionNormalizer {}

pub struct LogicalInteractionNormalizerState {
    pub dialect_registry: std::sync::Arc<tokio::sync::Mutex<crate::ir::interaction::dialect::DialectRegistry>>,
}

impl LogicalInteractionNormalizer {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatefulTransformation<LogicalInteractionNormalizerState> for LogicalInteractionNormalizer {
    fn apply(
        &mut self,
        workflow: &mut crate::ir::workflow::ActiveWorkflow,
        _nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
        global_state: &LogicalInteractionNormalizerState,
    ) {
        let mut reg = global_state.dialect_registry.blocking_lock();

        let Ok(interactions) = collect_logical_interactions(workflow, &mut reg) else {
            log::warn!("Failed collecting logical interactions.");
            return;
        };

        let mapped_interactions = interactions
            .into_iter()
            .flat_map(|i| {
                if i.dialect_type.base_type == crate::ir::interaction::dialect::logical_overlay::ID {
                    return vec![i];
                }

                let Ok((target_dialect, _)) = reg.plan_translation(
                    &i,
                    &crate::ir::interaction::dialect::DialectDescriptor {
                        base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                        constraints: std::collections::BTreeSet::new(),
                    },
                ) else {
                    log::warn!("Cannot Plan Translation to Logical Overlay: {:?}", i.dialect_type.base_type);
                    return Vec::new();
                };

                let translated_interactions = reg.try_translate(&i, &target_dialect);

                let Ok(translated_interactions) = translated_interactions else {
                    log::warn!("Cannot Translate to Logical Overlay: {:?}", i.dialect_type.base_type);
                    return Vec::new();
                };

                return translated_interactions;
            })
            .collect();

        if let Err(e) = distribute_logical_interactions(mapped_interactions, workflow, &mut reg) {
            log::warn!("Failure distributing logical interactions: {e}")
        }
    }
}

fn collect_logical_interactions(
    workflow: &mut crate::ir::workflow::ActiveWorkflow,
    dialect_registry: &mut crate::ir::interaction::dialect::DialectRegistry,
) -> Result<Vec<crate::ir::interaction::InteractionMapping>, crate::ir::interaction::InteractionError> {
    let mut port_collector = std::collections::BTreeMap::<
        crate::ir::interaction::dialect::DialectDescriptor,
        (
            Vec<(crate::ir::interaction::LogicalPortId, crate::ir::interaction::SourcePortMapping)>,
            Vec<(crate::ir::interaction::LogicalPortId, crate::ir::interaction::DestiantionPortMapping)>,
        ),
    >::new();
    for (cid, component) in &mut workflow.components() {
        let mut component = component.borrow_mut();
        let ports = component.logical_ports_mut();

        std::mem::take(&mut ports.logical_input_mapping)
            .into_iter()
            .for_each(|(port_id, port_mapping)| {
                port_collector.entry(port_mapping.dialect_type.clone()).or_default().1.push((
                    interaction::LogicalPortId {
                        component: cid.to_string(),
                        port: port_id,
                    },
                    port_mapping,
                ))
            });
        std::mem::take(&mut ports.logical_output_mapping)
            .into_iter()
            .for_each(|(port_id, port_mapping)| {
                port_collector.entry(port_mapping.dialect_type.clone()).or_default().0.push((
                    interaction::LogicalPortId {
                        component: cid.to_string(),
                        port: port_id.clone(),
                    },
                    port_mapping,
                ))
            });
    }

    Ok(port_collector
        .into_iter()
        .map(|(dialect, (source_ports, destination_ports))| dialect_registry.logical_ports_to_interaction(&dialect, source_ports, destination_ports))
        .collect::<Result<Vec<_>, crate::ir::interaction::InteractionError>>()?
        .into_iter()
        .flatten()
        .collect())
}

fn distribute_logical_interactions(
    mapped_interactions: Vec<crate::ir::interaction::InteractionMapping>,
    workflow: &mut crate::ir::workflow::ActiveWorkflow,
    dialect_registry: &mut crate::ir::interaction::dialect::DialectRegistry,
) -> Result<(), crate::ir::interaction::InteractionError> {
    let mut replacement_source_ports = std::collections::BTreeMap::<
        String,
        std::collections::BTreeMap<edgeless_api::function_instance::PortId, interaction::SourcePortMapping>,
    >::new();
    let mut replacement_destination_ports = std::collections::BTreeMap::<
        String,
        std::collections::BTreeMap<edgeless_api::function_instance::PortId, interaction::DestiantionPortMapping>,
    >::new();

    for i in mapped_interactions {
        let (s, d) = dialect_registry.logical_interaction_to_ports(i)?;
        for (port_id, source_spec) in s {
            replacement_source_ports
                .entry(port_id.component.clone())
                .or_default()
                .insert(port_id.port.clone(), source_spec);
        }
        for (port_id, dest_spec) in d {
            replacement_destination_ports
                .entry(port_id.component.clone())
                .or_default()
                .insert(port_id.port.clone(), dest_spec);
        }
    }

    for (cid, component) in &mut workflow.components() {
        let mut c = component.borrow_mut();
        let ports = c.logical_ports_mut();

        let inputs = replacement_destination_ports.remove(&cid.to_string()).unwrap_or_default();
        let outputs = replacement_source_ports.remove(&cid.to_string()).unwrap_or_default();

        for (input_port_id, input_port_spec) in inputs {
            ports.logical_input_mapping.insert(input_port_id, input_port_spec.clone());
        }

        for (ouput_port_id, output_port_spec) in outputs {
            ports.logical_output_mapping.insert(ouput_port_id, output_port_spec.clone());
        }
    }

    Ok(())
}
