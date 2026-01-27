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
    #[tracing::instrument(name = "colocation_optimizer", skip_all)]
    fn apply(&mut self, workflow: &mut crate::ir::workflow::ActiveWorkflow, _nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        for (c_id, c) in workflow.components() {
            let component = c.borrow_mut();
            optimize_component(c_id, &*component);
        }
    }
}

fn optimize_component(component_id: &str, component: &dyn crate::ir::logical_model::LogicalComponent) {
    let crate::ir::component::ScalingMode::Singleton = component.scaling_mode() else {
        return;
    };

    let port_weights = crate::ir::support::materialized_port_scoring::port_weights(&*component, INTEREST_PERIOD).unwrap_or_default();
    tracing::debug!("Migration Port Weights {component_id}: {port_weights:?}");

    if port_weights.is_empty() {
        return;
    }

    if port_weights.len() == 1 {
        optimize_component_single_active_port(component_id, &*component);
    } else {
        optimize_component_multiple_active_ports(component_id, component, port_weights);
    }
}

fn optimize_component_single_active_port(component_id: &str, component: &dyn crate::ir::logical_model::LogicalComponent) {
    for instance in component.instances().iter() {
        let Ok(mut maybe_c) = instance.try_borrow_mut() else {
            continue;
        };

        let crate::ir::PhysicalComponentState::Materialized(ci) = &*maybe_c else {
            continue;
        };

        tracing::debug!(
            "Migration Single Port {component_id}: {:?}, {:?}",
            ci.creation_time().elapsed(),
            crate::ir::support::materialized_port_scoring::trafic_locality(ci.as_ref(), INTEREST_PERIOD)
        );

        if ci.creation_time().elapsed() > INTEREST_PERIOD {
            let traffic_per_node = crate::ir::support::materialized_port_scoring::traffic_per_node(ci.as_ref(), INTEREST_PERIOD);

            if traffic_per_node.len() < 1 {
                continue;
            }

            let node_score = traffic_per_node.iter().find(|(node_id, _score)| *node_id == ci.id().node_id);

            if let Some((_, score)) = node_score {
                if score + 10 < traffic_per_node[0].1 {
                    tracing::info!("Detected suboptimal placement (single_port; local<remote): {component_id}. Will now attempt migration.");
                    maybe_c.plan_migration();
                }
            } else {
                tracing::info!("Detected suboptimal placement (single_port; no local traffic): {component_id}. Will now attempt migration.");
                maybe_c.plan_migration();
            }
        }
    }
}

fn optimize_component_multiple_active_ports(
    component_id: &str,
    component: &dyn crate::ir::logical_model::LogicalComponent,
    port_weights: Vec<(edgeless_api::function_instance::PortId, u8)>,
) {
    for instance in component.instances().iter() {
        let Ok(mut maybe_c) = instance.try_borrow_mut() else {
            continue;
        };
        let crate::ir::PhysicalComponentState::Materialized(ci) = &mut *maybe_c else {
            continue;
        };
        if ci.creation_time().elapsed() < INTEREST_PERIOD {
            return;
        }

        let port_link_costs: std::collections::HashMap<edgeless_api::function_instance::PortId, u8> =
            crate::ir::support::materialized_port_scoring::dynamic_port_link_cost(ci.as_mut(), INTEREST_PERIOD)
                .unwrap()
                .into_iter()
                .collect();

        tracing::debug!("Migration Multi Port: {component_id}. Weight: {port_weights:?}; Cost: {port_link_costs:?}");
        // Migrate if one port is more than REQUIRED_WEIGHT_DELTA percent more frequent than the following port
        // AND the link of the frequent port is more than REQUIRED_LINK_COST_DEKTA expensive than the less frequent port.
        for i in 0..port_weights.len() - 1 {
            let frequent_port_weight = port_weights[i].1;
            let infrequent_port_weight = port_weights[i + 1].1;
            if frequent_port_weight >= infrequent_port_weight + REQUIRED_WEIGHT_DELTA {
                let frequent_port_link_cost = *port_link_costs.get(&port_weights[i].0).unwrap();
                let infrequent_port_link_cost = *port_link_costs.get(&port_weights[i + 1].0).unwrap();
                if frequent_port_link_cost > infrequent_port_link_cost + REQUIRED_LINK_COST_DELTA {
                    tracing::info!("Detected suboptimal placement (multi_port): {component_id}. Will now attempt migration.");
                    maybe_c.plan_migration();
                    return;
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
