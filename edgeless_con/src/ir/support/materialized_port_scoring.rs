// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub fn port_weights(
    component: &dyn crate::ir::LogicalComponent,
    period: std::time::Duration,
) -> Option<Vec<(edgeless_api::function_instance::PortId, u8)>> {
    let mut materialized_instance_count = 0;
    let mut total_port_rate = 0.0;

    let number_of_inputs = component.logical_ports().logical_input_mapping.len();
    let number_of_outputs = component.logical_ports().logical_output_mapping.len();
    let number_of_ports = number_of_inputs + number_of_outputs;

    let mut data = Vec::with_capacity(number_of_ports);

    component.instances().iter().for_each(|i| {
        if let Some(c) = i.borrow().try_unpack_active() {
            if let Some(materialized) = c.materialized_state() {
                materialized_instance_count += 1;
                for (i_port, input) in &mut materialized.borrow_mut().materialized_ports().materialized_inputs {
                    if let Some(port_statistics) = &mut input.port_statistics {
                        if let Some(port_rate) = port_statistics.message_rate_abs(period) {
                            total_port_rate += port_rate;
                            data.push((i_port.clone(), port_rate));
                        }
                    }
                }
                for (o_port, output) in &mut materialized.borrow_mut().materialized_ports().materialized_outputs {
                    if let Some(port_statistics) = &mut output.port_statistics {
                        if let Some(port_rate) = port_statistics.message_rate_abs(period) {
                            total_port_rate += port_rate;
                            data.push((o_port.clone(), port_rate));
                        }
                    }
                }
            }
        }
    });

    if materialized_instance_count == 0 {
        return None;
    }

    let mut data_normalized: Vec<(edgeless_api::function_instance::PortId, u8)> = data
        .into_iter()
        .map(|(id, port_rate)| (id, ((port_rate / total_port_rate * 100.0).ceil() as u8).min(100u8)))
        .collect();

    // Weights sorted descending; Break ties by ordering the port_id ascending.
    data_normalized.sort_by(|a, b| {
        let rate_order = b.1.cmp(&a.1);
        if let std::cmp::Ordering::Equal = rate_order {
            a.0 .0.cmp(&b.0 .0)
        } else {
            rate_order
        }
    });

    Some(data_normalized)
}

// This should be calculated from the physical representation but that is currently no possible as the inputs are not physically mapped.
pub fn dynamic_port_link_cost(
    component_instance: &mut dyn crate::ir::PhysicalComponent,
    period: std::time::Duration,
) -> Option<Vec<(edgeless_api::function_instance::PortId, u8)>> {
    let mut materialized_instance_count = 0;
    let number_of_inputs = component_instance.physical_ports().physical_input_mapping.len();
    let number_of_outputs = component_instance.physical_ports().physical_output_mapping.len();
    let number_of_ports = number_of_inputs + number_of_outputs;

    let mut data = Vec::with_capacity(number_of_ports);

    let mut total_port_cost = 0.0;

    if let Some(materialized) = component_instance.materialized_state() {
        materialized_instance_count += 1;
        for (i_port, input) in &mut materialized.borrow_mut().materialized_ports().materialized_inputs {
            if let Some(port_statistics) = &mut input.port_statistics {
                let mut port_total_rate = 0.0;
                let peer_rates = port_statistics.message_rate_abs_by_peer(period);
                for (_peer_id, rate) in &peer_rates {
                    port_total_rate += rate;
                }

                let mut port_cost: f64 = 0.0;
                for (peer_id, rate) in &peer_rates {
                    let link_cost = if peer_id.node_id == component_instance.id().node_id {
                        1.0
                    } else {
                        100.0
                    };
                    port_cost += (rate / port_total_rate) * link_cost;
                }
                total_port_cost += port_cost;
                data.push((i_port.clone(), port_cost));
            }
        }
        for (o_port, output) in &mut materialized.borrow_mut().materialized_ports().materialized_outputs {
            if let Some(port_statistics) = &mut output.port_statistics {
                let mut port_total = 0.0;
                let peer_rates = port_statistics.message_rate_abs_by_peer(period);
                for (_peer_id, rate) in &peer_rates {
                    port_total += rate;
                }

                let mut port_cost: f64 = 0.0;
                for (peer_id, rate) in &peer_rates {
                    let link_cost = if peer_id.node_id == component_instance.id().node_id {
                        1.0
                    } else {
                        100.0
                    };

                    port_cost += (rate / port_total) * link_cost;
                    log::info!("{}: {} {}", o_port.0, link_cost, port_cost);
                }
                total_port_cost += port_cost;
                data.push((o_port.clone(), port_cost));
            }
        }
    }

    if materialized_instance_count == 0 {
        return None;
    }

    let mut data_normalized: Vec<(edgeless_api::function_instance::PortId, u8)> = data
        .into_iter()
        .map(|(id, port_cost)| (id, ((port_cost / total_port_cost * 100.0).ceil() as u8).min(100u8)))
        .collect();

    // Weights sorted descending; Break ties by ordering the port_id ascending.
    data_normalized.sort_by(|a, b| {
        let rate_order = b.1.cmp(&a.1);
        if let std::cmp::Ordering::Equal = rate_order {
            a.0 .0.cmp(&b.0 .0)
        } else {
            rate_order
        }
    });

    Some(data_normalized)
}

pub fn trafic_locality(component_instance: &dyn crate::ir::PhysicalComponent, period: std::time::Duration) -> u8 {
    let mut materialized_state = component_instance.materialized_state().unwrap().borrow_mut();
    let p = materialized_state.materialized_ports();

    let mut local_rate = 0.0f64;
    let mut total_rate = 0.0f64;

    for input in p.materialized_inputs.values() {
        for (peer_id, rate) in input.port_statistics.as_ref().unwrap().message_rate_abs_by_peer(period) {
            total_rate += rate;
            if peer_id.node_id == component_instance.id().node_id {
                local_rate += rate;
            }
        }
    }

    for output in p.materialized_outputs.values() {
        for (peer_id, rate) in output.port_statistics.as_ref().unwrap().message_rate_abs_by_peer(period) {
            total_rate += rate;
            if peer_id.node_id == component_instance.id().node_id {
                local_rate += rate;
            }
        }
    }

    100u8.min((local_rate / total_rate * 100.0).ceil() as u8)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ir::test::component_mock;
    use crate::ir::LogicalComponent;

    #[test]
    fn port_weights_different_rates() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 1.0), (remote_other_id, 9.0));
        let port_rates = port_weights(&component_under_test, std::time::Duration::from_secs(60));
        let port_rates = port_rates.unwrap();
        assert_eq!(port_rates.len(), 2);
        assert_eq!(
            vec![
                (edgeless_api::function_instance::PortId("output_2".to_string()), 90u8),
                (edgeless_api::function_instance::PortId("output_1".to_string()), 10u8),
            ],
            port_rates
        )
    }

    #[test]
    fn port_weights_different_rates_rounding() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 1.05), (remote_other_id, 9.05));
        let port_rates = port_weights(&component_under_test, std::time::Duration::from_secs(60));
        let port_rates = port_rates.unwrap();
        assert_eq!(port_rates.len(), 2);
        assert_eq!(
            vec![
                (edgeless_api::function_instance::PortId("output_2".to_string()), 90u8),
                (edgeless_api::function_instance::PortId("output_1".to_string()), 11u8),
            ],
            port_rates
        )
    }

    #[test]
    fn port_weights_equal_rates() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 5.0), (remote_other_id, 5.0));
        let port_rates = port_weights(&component_under_test, std::time::Duration::from_secs(60));
        let port_rates = port_rates.unwrap();
        assert_eq!(port_rates.len(), 2);
        assert_eq!(
            vec![
                (edgeless_api::function_instance::PortId("output_1".to_string()), 50u8),
                (edgeless_api::function_instance::PortId("output_2".to_string()), 50u8),
            ],
            port_rates
        )
    }

    #[test]
    fn port_link_cost_one_remote() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 5.0), (remote_other_id, 5.0));
        let port_link_costs = dynamic_port_link_cost(
            component_under_test.instances()[0].borrow_mut().try_unpack_materialized_mut().unwrap(),
            std::time::Duration::from_secs(60),
        );
        let port_link_costs = port_link_costs.unwrap();
        assert_eq!(port_link_costs.len(), 2);
        assert_eq!(
            vec![
                (edgeless_api::function_instance::PortId("output_2".to_string()), 100u8),
                (edgeless_api::function_instance::PortId("output_1".to_string()), 1u8),
            ],
            port_link_costs
        )
    }

    #[test]
    fn port_link_cost_both_local() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id2 = edgeless_api::function_instance::InstanceId::new(nodes[0]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 5.0), (colocated_other_id2, 5.0));
        let port_link_costs = dynamic_port_link_cost(
            component_under_test.instances()[0].borrow_mut().try_unpack_materialized_mut().unwrap(),
            std::time::Duration::from_secs(60),
        );
        let port_link_costs = port_link_costs.unwrap();
        assert_eq!(port_link_costs.len(), 2);
        assert_eq!(
            vec![
                (edgeless_api::function_instance::PortId("output_1".to_string()), 50u8),
                (edgeless_api::function_instance::PortId("output_2".to_string()), 50u8),
            ],
            port_link_costs
        )
    }

    #[test]
    fn port_link_cost_both_remote() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);
        let remote_other_id2 = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (remote_other_id2, 5.0), (remote_other_id, 5.0));
        let port_link_costs = dynamic_port_link_cost(
            component_under_test.instances()[0].borrow_mut().try_unpack_materialized_mut().unwrap(),
            std::time::Duration::from_secs(60),
        );
        let port_link_costs = port_link_costs.unwrap();
        assert_eq!(port_link_costs.len(), 2);
        assert_eq!(
            vec![
                (edgeless_api::function_instance::PortId("output_1".to_string()), 50u8),
                (edgeless_api::function_instance::PortId("output_2".to_string()), 50u8),
            ],
            port_link_costs
        )
    }

    #[test]
    fn traffic_locality_all_local() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id2 = edgeless_api::function_instance::InstanceId::new(nodes[0]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 5.0), (colocated_other_id2, 5.0));
        let port_link_costs = trafic_locality(
            component_under_test.instances()[0].borrow_mut().try_unpack_materialized_mut().unwrap(),
            std::time::Duration::from_secs(60),
        );
        assert_eq!(port_link_costs, 100);
    }

    #[test]
    fn traffic_locality_all_remote() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);
        let remote_other_id2 = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (remote_other_id, 5.0), (remote_other_id2, 5.0));
        let port_link_costs = trafic_locality(
            component_under_test.instances()[0].borrow_mut().try_unpack_materialized_mut().unwrap(),
            std::time::Duration::from_secs(60),
        );
        assert_eq!(port_link_costs, 0);
    }

    #[test]
    fn traffic_locality_split() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (remote_other_id, 5.0), (colocated_other_id, 5.0));
        let port_link_costs = trafic_locality(
            component_under_test.instances()[0].borrow_mut().try_unpack_materialized_mut().unwrap(),
            std::time::Duration::from_secs(60),
        );
        assert_eq!(port_link_costs, 50);
    }
}
