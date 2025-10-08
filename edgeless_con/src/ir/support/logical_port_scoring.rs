// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub fn peer_weights(
    component: &dyn crate::ir::LogicalComponent,
    port_weights: Vec<(edgeless_api::function_instance::PortId, u8)>,
) -> Vec<(String, u8)> {
    let port_weights: std::collections::HashMap<_, _> = port_weights.iter().map(|(port_id, weight)| (port_id.0.clone(), *weight)).collect();

    let mut logical_peers = std::collections::HashMap::<String, f64>::new();
    let mut total = 0.0f64;

    {
        for (input_id, input) in &component.logical_ports().logical_input_mapping {
            match input {
                crate::ir::LogicalInput::Direct(items) => {
                    let ilen = items.len() as f64;
                    for (item_id, _) in items {
                        let port_weight = *port_weights.get(&input_id.0).unwrap() as f64;
                        total += port_weight * 1.0f64 / ilen;
                        *logical_peers.entry(item_id.clone()).or_insert(0.0f64) += port_weight * 1.0f64 / ilen;
                    }
                }
                crate::ir::LogicalInput::Topic(_) => {
                    log::warn!("Topic Port occured after logical phase.")
                }
            }
        }

        for (output_id, output) in &component.logical_ports().logical_output_mapping {
            match output {
                edgeless_api::workflow_instance::PortMapping::DirectTarget(logical_id, _port_id) => {
                    let port_weight = *port_weights.get(&output_id.0).unwrap() as f64;
                    total += port_weight;
                    *logical_peers.entry(logical_id.clone()).or_insert(0.0f64) += port_weight;
                }
                edgeless_api::workflow_instance::PortMapping::AnyOfTargets(items) => {
                    for (logical_id, _port_id) in items {
                        let port_weight = *port_weights.get(&output_id.0).unwrap() as f64;
                        total += port_weight * 1.0f64 / items.len() as f64;
                        *logical_peers.entry(logical_id.clone()).or_insert(0.0f64) += port_weight * 1.0f64 / items.len() as f64;
                    }
                }
                edgeless_api::workflow_instance::PortMapping::AllOfTargets(items) => {
                    for (logical_id, _port_id) in items {
                        let port_weight = *port_weights.get(&output_id.0).unwrap() as f64;
                        total += port_weight;
                        *logical_peers.entry(logical_id.clone()).or_insert(0.0f64) += port_weight;
                    }
                }
                edgeless_api::workflow_instance::PortMapping::Topic(_) => {
                    log::warn!("Topic Port occured after logical phase.")
                }
            }
        }
    }

    let mut data_normalized: Vec<(String, u8)> = logical_peers
        .into_iter()
        .map(|(k, v)| (k, ((v / total * 100.0).ceil() as u8).min(100u8)))
        .collect();

    // Weights sorted descending; Break ties by ordering the port_id ascending.
    data_normalized.sort_by(|a, b| {
        let rate_order = b.1.cmp(&a.1);
        if let std::cmp::Ordering::Equal = rate_order {
            a.0.cmp(&b.0)
        } else {
            rate_order
        }
    });

    data_normalized
}

pub fn port_weights(component: &dyn crate::ir::LogicalComponent) -> Vec<(edgeless_api::function_instance::PortId, u8)> {
    let number_of_inputs = component.logical_ports().logical_input_mapping.len();
    let number_of_outputs = component.logical_ports().logical_output_mapping.len();
    let number_of_ports = number_of_inputs + number_of_outputs;

    let mut data = Vec::with_capacity(number_of_ports);

    for i_id in component.logical_ports().logical_input_mapping.keys() {
        data.push((i_id.clone(), (1.0 / number_of_ports as f64 * 100.0).ceil() as u8));
    }

    for o_id in component.logical_ports().logical_output_mapping.keys() {
        data.push((o_id.clone(), (1.0 / number_of_ports as f64 * 100.0).ceil() as u8));
    }

    // Weights sorted descending; Break ties by ordering the port_id ascending.
    data.sort_by(|a, b| {
        let rate_order = b.1.cmp(&a.1);
        if let std::cmp::Ordering::Equal = rate_order {
            a.0 .0.cmp(&b.0 .0)
        } else {
            rate_order
        }
    });

    data
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ir::test::component_mock;

    #[test]
    fn equal_peer_weights() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 5.0), (remote_other_id, 5.0));
        let port_weights = port_weights(&component_under_test);

        let weights = peer_weights(&component_under_test, port_weights);

        assert_eq!(weights.len(), 2);
        assert_eq!(vec![("other_1".to_string(), 50u8), ("other_2".to_string(), 50u8),], weights)
    }

    #[test]
    fn equal_port_weights() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 5.0), (remote_other_id, 5.0));
        let port_rates = port_weights(&component_under_test);
        assert_eq!(port_rates.len(), 2);
        assert_eq!(
            vec![
                (edgeless_api::function_instance::PortId("output_1".to_string()), 50u8),
                (edgeless_api::function_instance::PortId("output_2".to_string()), 50u8),
            ],
            port_rates
        )
    }
}
