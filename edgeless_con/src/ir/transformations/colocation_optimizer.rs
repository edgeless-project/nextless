// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct ColocationOptimizer {}

impl ColocationOptimizer {
    pub fn new() -> Self {
        Self {}
    }
}

const REQUIRED_WEIGHT_DELTA: u8 = 10;
const REQUIRED_LINK_COST_DELTA: u8 = 10;
const INTEREST_PERIOD: std::time::Duration = std::time::Duration::from_secs(60);

impl super::StatelessTransformation for ColocationOptimizer {
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, _nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        'each_component: for (c_id, c) in workflow.components() {
            let component = c.borrow_mut();
            let port_weights = crate::ir::support::materialized_port_scoring::port_weights(&*component, INTEREST_PERIOD).unwrap_or_default();
            log::info!("Migration Port Weights {c_id}: {port_weights:?}");

            if port_weights.is_empty() {
                continue 'each_component;
            }

            if port_weights.len() == 1 {
                for instance in component.instances().iter() {
                    if let Ok(mut maybe_c) = instance.try_borrow_mut() {
                        if let crate::ir::PhysicalComponentState::Materialized(ci) = &*maybe_c {
                            log::info!(
                                "Migration Single Port {c_id}: {:?}, {:?}",
                                ci.creation_time().elapsed(),
                                crate::ir::support::materialized_port_scoring::trafic_locality(ci.as_ref(), INTEREST_PERIOD)
                            );
                            if (ci.creation_time().elapsed() > INTEREST_PERIOD)
                                && crate::ir::support::materialized_port_scoring::trafic_locality(ci.as_ref(), INTEREST_PERIOD) < 95
                            {
                                log::info!("Detected suboptimal placement (single_port). Will now attempt migration.");
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
                        if ci.creation_time().elapsed() < INTEREST_PERIOD {
                            continue 'each_instance;
                        }

                        let port_link_costs: std::collections::HashMap<edgeless_api::function_instance::PortId, u8> =
                            crate::ir::support::materialized_port_scoring::dynamic_port_link_cost(ci.as_mut(), INTEREST_PERIOD)
                                .unwrap()
                                .into_iter()
                                .collect();

                        log::info!("Migration Multi Port. Weight: {port_weights:?}; Cost: {port_link_costs:?}");
                        // Migrate if one port is more than REQUIRED_WEIGHT_DELTA percent more frequent than the following port
                        // AND the link of the frequent port is more than REQUIRED_LINK_COST_DEKTA expensive than the less frequent port.
                        for i in 0..port_weights.len() - 1 {
                            let frequent_port_weight = port_weights[i].1;
                            let infrequent_port_weight = port_weights[i + 1].1;
                            if frequent_port_weight >= infrequent_port_weight + REQUIRED_WEIGHT_DELTA {
                                let frequent_port_link_cost = *port_link_costs.get(&port_weights[i].0).unwrap();
                                let infrequent_port_link_cost = *port_link_costs.get(&port_weights[i + 1].0).unwrap();
                                if frequent_port_link_cost > infrequent_port_link_cost + REQUIRED_LINK_COST_DELTA {
                                    log::info!("Detected suboptimal placement (multi_port). Will now attempt migration.");
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::ir::test::component_mock;
    use crate::ir::transformations::StatelessTransformation;

    #[test]
    fn migration_when_usefull() {
        let runtime = crate::ir::test::MockWasmRuntime {};
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
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
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
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
        let actor_image = crate::ir::test::mock_actor_image();
        let (nodes, _candidates) = crate::ir::test::mock_nodes_and_candidates(2, &runtime, actor_image.main_image.behavior_image_id);
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
}
