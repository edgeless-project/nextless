// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use crate::ir::interaction::dialect::InteractionDialect;

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

        let interactions = collect_physical_interactions(workflow, &mut reg);

        // https://stackoverflow.com/a/59852696
        //

        // : Result<Vec<interaction::InteractionMapping>, ()>
        let mapped_interactions = interactions
            .into_iter()
            .flat_map(|i| {
                let node_ids = i.mapping.as_physical().unwrap().relevant_nodes();

                let mut supprted_dialects = std::collections::BTreeMap::new();

                for node_id in node_ids {
                    if let Some(node) = nodes.get(&node_id) {
                        let node_dialects: std::collections::BTreeMap<_, _> = node
                            .available_interaction_dialects()
                            .into_iter()
                            .map(|d| (d.base_type, d.constraints))
                            .collect();

                        if supprted_dialects.is_empty() {
                            for (base_type, constraints) in &node_dialects {
                                supprted_dialects.insert(base_type.clone(), constraints.clone());
                            }
                        } else {
                            supprted_dialects.retain(|base_type, constraints| {
                                node_dialects
                                    .get(base_type)
                                    .is_some_and(|existing_constraints| existing_constraints == constraints)
                            });
                        }
                    } else {
                        log::warn!("Could not find node for mapping.");
                        return vec![];
                    }
                }

                let mut supported_dialects: Vec<_> = supprted_dialects.into_iter().collect();

                // TODO: Make this generic by moving it to the dialects.
                supported_dialects.sort_by(|(a_base, _a_constraints), (b_base, _b_constraints)| {
                    if a_base == &crate::ir::interaction::dialect::ip_multicast::ID
                        && b_base == &crate::ir::interaction::dialect::physical_overlay::ID
                    {
                        return std::cmp::Ordering::Less;
                    }

                    if b_base == &crate::ir::interaction::dialect::ip_multicast::ID
                        && a_base == &crate::ir::interaction::dialect::physical_overlay::ID
                    {
                        return std::cmp::Ordering::Greater;
                    }

                    return std::cmp::Ordering::Equal;
                });

                for (supported_dialect_base, supported_dialect_constraints) in supported_dialects {
                    let target = reg.plan_translation(
                        &i.dialect_type,
                        &crate::ir::interaction::dialect::DialectDescriptor {
                            base_type: supported_dialect_base,
                            constraints: supported_dialect_constraints,
                        },
                    );

                    let Ok(target) = target else {
                        log::debug!(
                            "Interaction translation plan failed: {:?} -> {:?}",
                            i.dialect_type.base_type,
                            supported_dialect_base
                        );
                        continue;
                    };

                    let translation = reg.try_translate(&i, &target);

                    if let Ok(translation) = translation {
                        return translation;
                    } else {
                        log::debug!(
                            "Interaction translation failed: {:?} -> {:?}",
                            i.dialect_type.base_type,
                            supported_dialect_base
                        );
                    }
                }

                log::warn!("Failed to map interaction; Falling back to old value.");
                vec![i.clone()]
            })
            .collect();

        for i in &mapped_interactions {
            let link_config = crate::ir::interaction::dialect::physical_overlay::PhysicalOverlayDialect {}.link_config(i, nodes);

            workflow.links.entry(link_config.id.clone()).or_insert(link_config);
        }

        distribute_physical_interactions(mapped_interactions, workflow, &mut reg);
    }
}

fn collect_physical_interactions(
    workflow: &mut crate::ir::workflow::ActiveWorkflow,
    dialect_registry: &mut crate::ir::interaction::dialect::DialectRegistry,
) -> Vec<crate::ir::interaction::InteractionMapping> {
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
            if let Some(i) = i.borrow_mut().try_unpack_materialized_mut() {
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

    port_collector
        .into_iter()
        .flat_map(|(dialect, (source_ports, destination_ports))| {
            dialect_registry
                .physical_ports_to_interaction(&dialect, source_ports, destination_ports)
                .unwrap()
        })
        .collect()
}

fn distribute_physical_interactions(
    mapped_interactions: Vec<crate::ir::interaction::InteractionMapping>,
    workflow: &mut crate::ir::workflow::ActiveWorkflow,
    dialect_registry: &mut crate::ir::interaction::dialect::DialectRegistry,
) {
    let mut replacement_srcs = std::collections::BTreeMap::<
        edgeless_api::function_instance::InstanceId,
        std::collections::BTreeMap<edgeless_api::function_instance::PortId, interaction::SourcePortMapping>,
    >::new();
    let mut replacement_dests = std::collections::BTreeMap::<
        edgeless_api::function_instance::InstanceId,
        std::collections::BTreeMap<edgeless_api::function_instance::PortId, interaction::DestiantionPortMapping>,
    >::new();

    for i in mapped_interactions {
        let (s, d) = dialect_registry.physical_interaction_to_ports(i).unwrap();
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
            if let Some(i) = i.borrow_mut().try_unpack_materialized_mut() {
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
}
