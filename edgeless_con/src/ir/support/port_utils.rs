// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub fn collect_physical_interactions(
    workflow: &crate::ir::workflow::ActiveWorkflow,
    dialect_registry: &mut crate::ir::interaction::dialect::DialectRegistry,
) -> Result<Vec<crate::ir::interaction::InteractionMapping>, crate::ir::interaction::InteractionError> {
    let mut port_collector = std::collections::BTreeMap::<
        crate::ir::interaction::dialect::DialectDescriptor,
        (
            Vec<(crate::ir::interaction::PhysicalPortId, crate::ir::interaction::SourcePortMapping)>,
            Vec<(crate::ir::interaction::PhysicalPortId, crate::ir::interaction::DestiantionPortMapping)>,
        ),
    >::new();

    for (_c_id, _logical_component, instances) in workflow.components_with_instances() {
        for i in instances {
            if let Some(i) = i.component.try_unpack_active() {
                let cloned_id = i.id().clone();
                let ports = i.physical_ports();

                for (out_id, out) in &ports.physical_output_mapping {
                    port_collector.entry(out.dialect_type.clone()).or_default().0.push((
                        crate::ir::interaction::PhysicalPortId {
                            instance: cloned_id.clone(),
                            port: out_id.clone(),
                        },
                        out.clone(),
                    ));
                }

                for (input_id, input) in &ports.physical_input_mapping {
                    port_collector.entry(input.dialect_type.clone()).or_default().1.push((
                        crate::ir::interaction::PhysicalPortId {
                            instance: cloned_id.clone(),
                            port: input_id.clone(),
                        },
                        input.clone(),
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

pub fn distribute_physical_interactions(
    mapped_interactions: Vec<crate::ir::interaction::InteractionMapping>,
    workflow: &crate::ir::workflow::ActiveWorkflow,
    dialect_registry: &mut crate::ir::interaction::dialect::DialectRegistry,
) -> Result<Vec<crate::ir::transformations::PhysicalChange>, crate::ir::interaction::InteractionError> {
    let mut replacement_srcs = std::collections::BTreeMap::<
        edgeless_api::function_instance::InstanceId,
        std::collections::BTreeMap<edgeless_api::function_instance::PortId, crate::ir::interaction::SourcePortMapping>,
    >::new();
    let mut replacement_dests = std::collections::BTreeMap::<
        edgeless_api::function_instance::InstanceId,
        std::collections::BTreeMap<edgeless_api::function_instance::PortId, crate::ir::interaction::DestiantionPortMapping>,
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

    let mut required_changes = Vec::new();

    for (_c_id, _logical_component, component_instances) in workflow.components_with_instances() {
        for i in component_instances {
            let mut cloned_instance = i.component.clone();
            let mut changed = false;

            if let Some(active_instance) = cloned_instance.try_unpack_active_mut() {
                let cloned_id = active_instance.id();
                let ports = active_instance.physical_ports_mut();

                let inputs = replacement_dests.remove(&cloned_id).unwrap_or_default();
                let outputs = replacement_srcs.remove(&cloned_id).unwrap_or_default();

                for (input_port, port_mapping) in inputs {
                    let existing_value = ports.physical_input_mapping.insert(input_port, port_mapping.clone());

                    if existing_value.is_none_or(|existing_value| existing_value != port_mapping) {
                        changed = true;
                    }
                }

                for (output_port, port_mapping) in outputs {
                    let existing_value = ports.physical_output_mapping.insert(output_port, port_mapping.clone());

                    if existing_value.is_none_or(|existing_value| existing_value != port_mapping) {
                        changed = true;
                    }
                }

                if changed {
                    required_changes.push(crate::ir::transformations::PhysicalChange::Component(
                        crate::ir::transformations::PhysicalComponentChange {
                            component_id: i.component_id,
                            action: crate::ir::transformations::PhysicalComponentChangeAction::Update(cloned_instance),
                        },
                    ))
                }
            }
        }
    }

    Ok(required_changes)
}
