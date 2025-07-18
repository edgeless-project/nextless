// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use std::time::Duration;

pub struct ColocationOptimizer {}

impl ColocationOptimizer {
    pub fn new() -> Self {
        Self {}
    }
}

const REQUIRED_WEIGHT_DELTA: u8 = 10;
const REQUIRED_LINK_COST_DELTA: u8 = 10;

impl super::StatelessTransformation for ColocationOptimizer {
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, _nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        'each_component: for (_c_id, c) in workflow.components() {
            let component = c.borrow_mut();
            let port_weights = dynamic_port_weight(&*component).unwrap_or_default();
            log::info!("Migration State1: {port_weights:?}");

            if port_weights.is_empty() {
                continue 'each_component;
            }

            if port_weights.len() == 1 {
                for instance in component.instances().iter() {
                    if let Ok(mut maybe_c) = instance.try_borrow_mut() {
                        if let crate::ir::PhysicalComponentState::Materialized(ci) = &mut *maybe_c {
                            if (ci.creation_time().elapsed() < Duration::from_secs(60)) && !all_traffic_local(ci.as_mut()) {
                                maybe_c.plan_migration();
                            }
                        }
                    }
                }

                continue 'each_component;
            }

            'each_instance: for instance in component.instances().iter() {
                if let Ok(mut maybe_c) = instance.try_borrow_mut() {
                    if let crate::ir::PhysicalComponentState::Materialized(ci) = &mut *maybe_c {
                        if ci.creation_time().elapsed() < Duration::from_secs(60) {
                            continue 'each_instance;
                        }

                        let port_link_costs: std::collections::HashMap<edgeless_api::function_instance::PortId, u8> =
                            dynamic_port_link_cost(ci.as_mut()).unwrap().into_iter().collect();

                        log::info!("Migration State2: {port_weights:?}, {port_link_costs:?}");
                        // Migrate if one port is more than REQUIRED_WEIGHT_DELTA percent more frequent than the following port
                        // AND the link of the frequent port is more than REQUIRED_LINK_COST_DEKTA expensive than the less frequent port.
                        for i in 0..port_weights.len() - 1 {
                            let frequent_port_weight = port_weights[i].1;
                            let infrequent_port_weight = port_weights[i + 1].1;
                            if frequent_port_weight >= infrequent_port_weight + REQUIRED_WEIGHT_DELTA {
                                let frequent_port_link_cost = *port_link_costs.get(&port_weights[i].0).unwrap();
                                let infrequent_port_link_cost = *port_link_costs.get(&port_weights[i + 1].0).unwrap();
                                if frequent_port_link_cost > infrequent_port_link_cost + REQUIRED_LINK_COST_DELTA {
                                    log::info!("Detected suboptimal placement. Will now attempt migration.");
                                    maybe_c.plan_migration();
                                    continue 'each_component;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn dynamic_port_weight(component: &dyn crate::ir::LogicalComponent) -> Option<Vec<(edgeless_api::function_instance::PortId, u8)>> {
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
                        if let Some(port_rate) = port_statistics.message_rate_abs(Duration::from_secs(60)) {
                            total_port_rate += port_rate;
                            data.push((i_port.clone(), port_rate));
                        }
                    }
                }
                for (o_port, output) in &mut materialized.borrow_mut().materialized_ports().materialized_outputs {
                    if let Some(port_statistics) = &mut output.port_statistics {
                        if let Some(port_rate) = port_statistics.message_rate_abs(Duration::from_secs(60)) {
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
fn dynamic_port_link_cost(component_instance: &mut dyn crate::ir::PhysicalComponent) -> Option<Vec<(edgeless_api::function_instance::PortId, u8)>> {
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
                let peer_rates = port_statistics.message_rate_abs_by_peer(Duration::from_secs(60));
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

                // data.push((i_port.clone(), (port_cost * 100.0).ceil() as u8));
            }
        }
        for (o_port, output) in &mut materialized.borrow_mut().materialized_ports().materialized_outputs {
            if let Some(port_statistics) = &mut output.port_statistics {
                let mut port_total = 0.0;
                let peer_rates = port_statistics.message_rate_abs_by_peer(Duration::from_secs(60));
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

fn all_traffic_local(component_instance: &mut dyn crate::ir::PhysicalComponent) -> bool {
    let mut materialized_state = component_instance.materialized_state().unwrap().borrow_mut();
    let p = materialized_state.materialized_ports();

    for input in p.materialized_inputs.values() {
        for (peer_id, _rate) in input.port_statistics.as_ref().unwrap().message_rate_abs_by_peer(Duration::from_secs(60)) {
            if peer_id.node_id != component_instance.id().node_id {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ir::{transformations::StatelessTransformation, LogicalComponent};

    #[test]
    fn migration_when_usefull() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 1.0), (remote_other_id, 9.0));
        let mut workflow_under_test = crate::ir::test::mock_workflow(std::collections::HashMap::from([(
            "test".to_string(),
            std::cell::RefCell::new(component_under_test),
        )]));

        let mut optimizer = ColocationOptimizer::new();
        optimizer.apply(
            &mut workflow_under_test,
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
        );

        let instances = &mut workflow_under_test.functions.get("test").unwrap().borrow_mut().instances;
        assert_eq!(instances.len(), 1);
        let instance = instances[0].borrow_mut();
        if let crate::ir::PhysicalComponentState::MigrationRequested(instance) = &*instance {
            assert_eq!(instance.id(), component_id);
        } else {
            panic!();
        }
    }

    #[test]
    fn no_migration_if_port_rate_equal() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 5.0), (remote_other_id, 5.0));
        let mut workflow_under_test = crate::ir::test::mock_workflow(std::collections::HashMap::from([(
            "test".to_string(),
            std::cell::RefCell::new(component_under_test),
        )]));

        let mut optimizer = ColocationOptimizer::new();
        optimizer.apply(
            &mut workflow_under_test,
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
        );

        let instances = &mut workflow_under_test.functions.get("test").unwrap().borrow_mut().instances;
        assert_eq!(instances.len(), 1);
        let instance = instances[0].borrow_mut();
        if let crate::ir::PhysicalComponentState::Materialized(instance) = &*instance {
            assert_eq!(instance.id(), component_id);
        } else {
            panic!();
        }
    }

    #[test]
    fn no_migration_if_link_cost_equal() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);
        let remote_other_id2 = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (remote_other_id2, 1.0), (remote_other_id, 9.0));
        let mut workflow_under_test = crate::ir::test::mock_workflow(std::collections::HashMap::from([(
            "test".to_string(),
            std::cell::RefCell::new(component_under_test),
        )]));

        let mut optimizer = ColocationOptimizer::new();
        optimizer.apply(
            &mut workflow_under_test,
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
        );

        let instances = &mut workflow_under_test.functions.get("test").unwrap().borrow_mut().instances;
        assert_eq!(instances.len(), 1);
        let instance = instances[0].borrow_mut();
        if let crate::ir::PhysicalComponentState::Materialized(instance) = &*instance {
            assert_eq!(instance.id(), component_id);
        } else {
            panic!();
        }
    }

    #[test]
    fn port_weights_different_rates() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 1.0), (remote_other_id, 9.0));
        let port_rates = dynamic_port_weight(&component_under_test);
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
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 1.05), (remote_other_id, 9.05));
        let port_rates = dynamic_port_weight(&component_under_test);
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
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 5.0), (remote_other_id, 5.0));
        let port_rates = dynamic_port_weight(&component_under_test);
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
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 5.0), (remote_other_id, 5.0));
        let port_link_costs = dynamic_port_link_cost(component_under_test.instances()[0].borrow_mut().try_unpack_materialized_mut().unwrap());
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
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let colocated_other_id2 = edgeless_api::function_instance::InstanceId::new(nodes[0]);

        let component_under_test = component_mock(component_id, (colocated_other_id, 5.0), (colocated_other_id2, 5.0));
        let port_link_costs = dynamic_port_link_cost(component_under_test.instances()[0].borrow_mut().try_unpack_materialized_mut().unwrap());
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
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime);
        let component_id = edgeless_api::function_instance::InstanceId::new(nodes[0]);
        let remote_other_id = edgeless_api::function_instance::InstanceId::new(nodes[1]);
        let remote_other_id2 = edgeless_api::function_instance::InstanceId::new(nodes[1]);

        let component_under_test = component_mock(component_id, (remote_other_id2, 5.0), (remote_other_id, 5.0));
        let port_link_costs = dynamic_port_link_cost(component_under_test.instances()[0].borrow_mut().try_unpack_materialized_mut().unwrap());
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

    fn component_mock(
        component_id: edgeless_api::function_instance::InstanceId,
        output_1: (edgeless_api::function_instance::InstanceId, f64),
        output_2: (edgeless_api::function_instance::InstanceId, f64),
    ) -> crate::ir::actor::LogicalActor {
        let cut_logical_ports = crate::ir::LogicalPorts {
            logical_output_mapping: std::collections::HashMap::from([
                (
                    edgeless_api::function_instance::PortId("output_1".to_string()),
                    edgeless_api::workflow_instance::PortMapping::DirectTarget(
                        "other_1".to_string(),
                        edgeless_api::function_instance::PortId("input_1".to_string()),
                    ),
                ),
                (
                    edgeless_api::function_instance::PortId("output_2".to_string()),
                    edgeless_api::workflow_instance::PortMapping::DirectTarget(
                        "other_2".to_string(),
                        edgeless_api::function_instance::PortId("input_1".to_string()),
                    ),
                ),
            ]),
            logical_input_mapping: std::collections::HashMap::new(),
        };

        let instance_1 = (
            component_id,
            crate::ir::MaterializedPorts {
                materialized_outputs: std::collections::HashMap::from([
                    (
                        edgeless_api::function_instance::PortId("output_1".to_string()),
                        crate::ir::MaterializedOutput {
                            mapping: edgeless_api::common::Output::Single(
                                output_1.0.clone(),
                                edgeless_api::function_instance::PortId("input_1".to_string()),
                            ),
                            port_statistics: Some(Box::new(crate::ir::test::MockPortStats::new(
                                std::collections::HashMap::from([output_1.clone()]),
                                std::collections::HashMap::new(),
                            ))),
                        },
                    ),
                    (
                        edgeless_api::function_instance::PortId("output_2".to_string()),
                        crate::ir::MaterializedOutput {
                            mapping: edgeless_api::common::Output::Single(
                                output_2.0.clone(),
                                edgeless_api::function_instance::PortId("input_1".to_string()),
                            ),
                            port_statistics: Some(Box::new(crate::ir::test::MockPortStats::new(
                                std::collections::HashMap::from([output_2.clone()]),
                                std::collections::HashMap::new(),
                            ))),
                        },
                    ),
                ]),
                materialized_inputs: std::collections::HashMap::new(),
            },
        );

        crate::ir::test::new_actor_with_mocked_materialized_instances(cut_logical_ports, vec![instance_1])
    }
}
