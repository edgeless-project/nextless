// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct Scaler {}

impl Scaler {
    pub fn new() -> Self {
        Self {}
    }
}

const UPDATE_RATE_SECS: u64 = 30;

// Probably should only be called "LoadScaler"
impl super::StatelessPhysicalTransformation for Scaler {
    #[tracing::instrument(name = "scaler", skip_all)]
    fn apply(
        &mut self,
        workflow: &crate::ir::workflow::ActiveWorkflow,
        available_nodes: &crate::ir::Nodes,
        _peer_clusters: &crate::ir::Clusters,
    ) -> Vec<super::PhysicalChange> {
        let mut required_changes = Vec::new();

        for (logical_component_id, component, instances) in workflow.components_with_instances() {
            // Skipped as this is not fully implemented yet.
            if logical_component_id == "__proxy" {
                continue;
            }

            match component.scaling_mode() {
                crate::ir::component::ScalingMode::Singleton { .. } => {
                    required_changes.extend(scale_singleton(logical_component_id, component, instances));
                }
                crate::ir::component::ScalingMode::Scalable { .. } => {
                    required_changes.extend(scale_scalable(logical_component_id, component, instances));
                }
                crate::ir::component::ScalingMode::AllNodes { .. } => {
                    required_changes.extend(scale_to_all_nodes(logical_component_id, component, instances, available_nodes));
                }
            }
        }

        required_changes
    }
}

fn scale_to_all_nodes(
    logical_function_id: &str,
    f: &crate::ir::logical_model::LogicalComponent,
    instances: crate::ir::workflow::InstanceIterator,
    available_nodes: &crate::ir::Nodes,
) -> Vec<super::PhysicalChange> {
    let mut required_changes = Vec::new();

    let mut covered_nodes = std::collections::HashSet::<uuid::Uuid>::new();

    for component_instance in instances {
        if let Some(active_actor) = component_instance.component.try_unpack_active() {
            covered_nodes.insert(active_actor.id().node_id.clone());
        }
    }

    let available_allowed_node_ids = available_nodes
        .iter()
        .filter(|(_id, node)| super::placement::feasibility::node_fulfills_constraints(&f.node_filters(), **node))
        .map(|(id, _node)| id.clone())
        .collect::<std::collections::HashSet<_>>();

    let missing_nodes = available_allowed_node_ids.difference(&covered_nodes);

    for missing_node in missing_nodes {
        tracing::debug!("AllNode Actor {logical_function_id}: Spawning instance cover node {missing_node} ");

        let mut node_filter = f.node_filters();
        node_filter.node_ids_allowed = Some(vec![missing_node.clone()]);

        let (component_id, component) = super::super::PhysicalComponentState::request_new_instance_with_extra_constraints(node_filter);

        required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
            component_id,
            action: super::PhysicalComponentChangeAction::Insert(logical_function_id.to_string(), component),
        }));
    }

    required_changes
}

fn scale_singleton(
    logical_function_id: &str,
    _logical_component: &crate::ir::logical_model::LogicalComponent,
    instances: crate::ir::workflow::InstanceIterator,
) -> Vec<super::PhysicalChange> {
    for physical_instance in instances {
        if physical_instance.component.try_unpack_active().is_some() {
            return vec![];
        }
    }

    tracing::debug!("Singleton Actor {logical_function_id}: Spawning missing instance");

    let (component_id, component) = super::super::PhysicalComponentState::request_new_instance();

    vec![super::PhysicalChange::Component(super::PhysicalComponentChange {
        component_id,
        action: super::PhysicalComponentChangeAction::Insert(logical_function_id.to_string(), component),
    })]
}

fn scale_scalable(
    logical_function_id: &str,
    f: &crate::ir::logical_model::LogicalComponent,
    instances: crate::ir::workflow::InstanceIterator,
) -> Vec<super::PhysicalChange> {
    let mut required_changes = Vec::new();

    let crate::ir::component::ScalingMode::Scalable {
        min_instances,
        max_instances,
        ..
    } = &f.scaling_mode()
    else {
        return vec![];
    };

    let active_instance_count = instances.clone().filter_active().count();

    let missing_instance_count = if *min_instances > active_instance_count {
        min_instances - active_instance_count
    } else {
        0
    };

    if missing_instance_count > 0 {
        tracing::debug!("Scalable Actor {logical_function_id}: Spawning {missing_instance_count} instances to reach min_instances");
        for _i in 0..missing_instance_count {
            let (component_id, component) = super::super::PhysicalComponentState::request_new_instance();
            required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
                component_id,
                action: super::PhysicalComponentChangeAction::Insert(logical_function_id.to_string(), component),
            }));
        }
        return required_changes;
    }

    let can_scale_up = active_instance_count <= *max_instances;

    let mut processing_rate = 0.0;
    let mut message_rate = 0.0;
    let mut last_start: Option<std::time::Instant> = None;

    for i in instances {
        if let super::super::PhysicalComponentState::Materialized(instance) = &*i.component {
            if let Some(l) = last_start {
                last_start = Some(l.max(instance.creation_time()))
            } else {
                last_start = Some(instance.creation_time())
            }
            if let Some(materialized) = &instance.materialized_state() {
                if let Some(rt) = &materialized.runtime_statistics() {
                    let instance_rate = rt.invocation_rate_abs(std::time::Duration::from_secs(UPDATE_RATE_SECS)).unwrap_or(0f64);
                    processing_rate += instance_rate;
                }

                for p in materialized.materialized_ports().materialized_inputs.values() {
                    if let Some(stats) = &p.port_statistics {
                        message_rate += stats.message_rate_abs(std::time::Duration::from_secs(UPDATE_RATE_SECS)).unwrap_or(0f64)
                    }
                }
            }
        }
    }

    tracing::trace!("Function {logical_function_id}: Processing Rate: {processing_rate}, Message Rate: {message_rate}");

    let should_scale_up = message_rate > 1.1 * processing_rate;

    // let should_scale_down = processing_rate < number_of_existing_instances as f64;
    let wait_period_exceeded = if let Some(last_start) = last_start {
        last_start.elapsed() > std::time::Duration::from_secs(UPDATE_RATE_SECS)
    } else {
        false
    };

    if can_scale_up && should_scale_up && wait_period_exceeded {
        tracing::debug!("Attempting to Scale Up. Message Rate:{message_rate} Processing Rate:{processing_rate}");
        let (component_id, component) = super::super::PhysicalComponentState::request_new_instance();
        required_changes.push(super::PhysicalChange::Component(super::PhysicalComponentChange {
            component_id,
            action: super::PhysicalComponentChangeAction::Insert(logical_function_id.to_string(), component),
        }));
    }

    // if should_scale_down {
    //     f.instances.last().unwrap().borrow_mut().plan_stop();
    // }
    //
    required_changes
}

#[cfg(test)]
mod test {
    use crate::ir::transformations::StatelessPhysicalTransformation;

    #[test]
    fn scale_missing_singleton_actor() {
        let (logical_component_id, logical_component_under_test) = crate::ir::actor::mock_actor::MockActorBuilder::default()
            .with_scaling_mode(crate::ir::component::ScalingMode::Singleton)
            .build();

        let workflow_under_test = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&logical_component_id, &logical_component_under_test, &[])
            .build();

        let example_node_id = uuid::Uuid::new_v4();

        let mock_node = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id.clone())
            .build()
            .unwrap();

        let nodes = std::collections::HashMap::from([(example_node_id.clone(), &mock_node as &dyn crate::ir::Node)]);

        let mut scaler = super::Scaler::new();
        let changes = scaler.apply(&workflow_under_test, &nodes, &crate::ir::Clusters::default());

        assert_eq!(changes.len(), 1);

        let change = changes[0].clone();

        let crate::ir::transformations::PhysicalChange::Component(component_change) = change else {
            panic!("Not the expected change");
        };

        let crate::ir::transformations::PhysicalComponentChangeAction::Insert(logical_id, _instance) = component_change.action else {
            panic!("Not the expected change");
        };

        assert_eq!("test", logical_id.as_str());
    }

    #[test]
    fn dont_update_existing_singleton_actor() {
        let (logical_component_id, logical_component_under_test) = crate::ir::actor::mock_actor::MockActorBuilder::default()
            .with_scaling_mode(crate::ir::component::ScalingMode::Singleton)
            .build();

        let (_, physical_component_id, physical_component) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&logical_component_id, &logical_component_under_test).build();

        let workflow_under_test = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(
                &logical_component_id,
                &logical_component_under_test,
                &[(physical_component_id, physical_component)],
            )
            .build();

        let example_node_id = uuid::Uuid::new_v4();

        let mock_node = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id.clone())
            .build()
            .unwrap();

        let nodes = std::collections::HashMap::from([(example_node_id.clone(), &mock_node as &dyn crate::ir::Node)]);

        let mut scaler = super::Scaler::new();
        let changes = scaler.apply(&workflow_under_test, &nodes, &crate::ir::Clusters::default());

        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn scale_missing_all_nodes_actor_allowed_node_filter() {
        let example_node_id = uuid::Uuid::new_v4();
        let example_node_id_2 = uuid::Uuid::new_v4();
        let example_node_id_3 = uuid::Uuid::new_v4();

        let (logical_component_id, logical_component_under_test) = crate::ir::actor::mock_actor::MockActorBuilder::default()
            .with_scaling_mode(crate::ir::component::ScalingMode::AllNodes)
            .with_node_filters(crate::ir::component::NodeFilters {
                node_ids_allowed: Some(vec![example_node_id, example_node_id_2]),
                ..Default::default()
            })
            .build();

        let workflow_under_test = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&logical_component_id, &logical_component_under_test, &[])
            .build();

        let mock_node = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id.clone())
            .build()
            .unwrap();

        let mock_node_2 = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id_2.clone())
            .build()
            .unwrap();

        let mock_node_3 = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id_3.clone())
            .build()
            .unwrap();

        let nodes = std::collections::HashMap::from([
            (example_node_id.clone(), &mock_node as &dyn crate::ir::Node),
            (example_node_id_2.clone(), &mock_node_2 as &dyn crate::ir::Node),
            (example_node_id_3.clone(), &mock_node_3 as &dyn crate::ir::Node),
        ]);

        let mut scaler = super::Scaler::new();
        let changes = scaler.apply(&workflow_under_test, &nodes, &crate::ir::Clusters::default());

        assert_eq!(changes.len(), 2);

        for change in changes {
            let crate::ir::transformations::PhysicalChange::Component(component_change) = change else {
                panic!("Not the expected change");
            };

            let crate::ir::transformations::PhysicalComponentChangeAction::Insert(logical_id, _instance) = component_change.action else {
                panic!("Not the expected change");
            };

            assert_eq!("test", logical_id.as_str());
        }
    }

    #[test]
    fn scale_missing_all_nodes_actor_label_filter() {
        let example_node_id = uuid::Uuid::new_v4();
        let example_node_id_2 = uuid::Uuid::new_v4();
        let example_node_id_3 = uuid::Uuid::new_v4();

        let (logical_component_id, logical_component_under_test) = crate::ir::actor::mock_actor::MockActorBuilder::default()
            .with_scaling_mode(crate::ir::component::ScalingMode::AllNodes)
            .with_node_filters(crate::ir::component::NodeFilters {
                node_label_filter_allowed: Some(vec![std::collections::HashSet::from(["testlabel".to_string()])]),
                ..Default::default()
            })
            .build();

        let workflow_under_test = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&logical_component_id, &logical_component_under_test, &[])
            .build();

        let mock_node = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id.clone())
            .labels(vec!["testlabel".to_string()])
            .build()
            .unwrap();

        let mock_node_2 = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id_2.clone())
            .labels(vec!["testlabel".to_string()])
            .build()
            .unwrap();

        let mock_node_3 = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id_3.clone())
            .build()
            .unwrap();

        let nodes = std::collections::HashMap::from([
            (example_node_id.clone(), &mock_node as &dyn crate::ir::Node),
            (example_node_id_2.clone(), &mock_node_2 as &dyn crate::ir::Node),
            (example_node_id_3.clone(), &mock_node_3 as &dyn crate::ir::Node),
        ]);

        let mut scaler = super::Scaler::new();
        let changes = scaler.apply(&workflow_under_test, &nodes, &crate::ir::Clusters::default());

        assert_eq!(changes.len(), 2);

        for change in changes {
            let crate::ir::transformations::PhysicalChange::Component(component_change) = change else {
                panic!("Not the expected change");
            };

            let crate::ir::transformations::PhysicalComponentChangeAction::Insert(logical_id, _instance) = component_change.action else {
                panic!("Not the expected change");
            };

            assert_eq!("test", logical_id.as_str());
        }
    }

    #[test]
    fn dont_scale_saturated_all_nodes_actor_label_filter() {
        let example_node_id = uuid::Uuid::new_v4();
        let example_node_id_2 = uuid::Uuid::new_v4();
        let example_node_id_3 = uuid::Uuid::new_v4();

        let (logical_component_id, logical_component_under_test) = crate::ir::actor::mock_actor::MockActorBuilder::default()
            .with_scaling_mode(crate::ir::component::ScalingMode::AllNodes)
            .with_node_filters(crate::ir::component::NodeFilters {
                node_label_filter_allowed: Some(vec![std::collections::HashSet::from(["testlabel".to_string()])]),
                ..Default::default()
            })
            .build();

        let (_, physical_component_id_1, physical_component_1) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&logical_component_id, &logical_component_under_test)
                .with_node_id(example_node_id)
                .build();

        let (_, physical_component_id_2, physical_component_2) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&logical_component_id, &logical_component_under_test)
                .with_node_id(example_node_id_2)
                .build();

        let workflow_under_test = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(
                &logical_component_id,
                &logical_component_under_test,
                &[
                    (physical_component_id_1, physical_component_1),
                    (physical_component_id_2, physical_component_2),
                ],
            )
            .build();

        let mock_node = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id.clone())
            .labels(vec!["testlabel".to_string()])
            .build()
            .unwrap();

        let mock_node_2 = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id_2.clone())
            .labels(vec!["testlabel".to_string()])
            .build()
            .unwrap();

        let mock_node_3 = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id_3.clone())
            .build()
            .unwrap();

        let nodes = std::collections::HashMap::from([
            (example_node_id.clone(), &mock_node as &dyn crate::ir::Node),
            (example_node_id_2.clone(), &mock_node_2 as &dyn crate::ir::Node),
            (example_node_id_3.clone(), &mock_node_3 as &dyn crate::ir::Node),
        ]);

        let mut scaler = super::Scaler::new();
        let changes = scaler.apply(&workflow_under_test, &nodes, &crate::ir::Clusters::default());

        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn scale_missing_scalable_actor() {
        let example_node_id = uuid::Uuid::new_v4();
        let example_node_id_2 = uuid::Uuid::new_v4();

        let (logical_component_id, logical_component_under_test) = crate::ir::actor::mock_actor::MockActorBuilder::default()
            .with_scaling_mode(crate::ir::component::ScalingMode::Scalable {
                min_instances: 1,
                max_instances: 2,
            })
            .build();

        let (physical_component_id_1, physical_component_1) =
            mock_scalable_instance(&logical_component_id, &logical_component_under_test, example_node_id.clone(), 2.0, 1.0);

        let workflow_under_test = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(
                &logical_component_id,
                &logical_component_under_test,
                &[(physical_component_id_1, physical_component_1)],
            )
            .build();

        let mock_node = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id.clone())
            .build()
            .unwrap();

        let mock_node_2 = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id_2.clone())
            .build()
            .unwrap();

        let nodes = std::collections::HashMap::from([
            (example_node_id.clone(), &mock_node as &dyn crate::ir::Node),
            (example_node_id_2.clone(), &mock_node_2 as &dyn crate::ir::Node),
        ]);

        let mut scaler = super::Scaler::new();
        let changes = scaler.apply(&workflow_under_test, &nodes, &crate::ir::Clusters::default());

        assert_eq!(changes.len(), 1);

        let change = changes[0].clone();

        let crate::ir::transformations::PhysicalChange::Component(component_change) = change else {
            panic!("Not the expected change");
        };

        let crate::ir::transformations::PhysicalComponentChangeAction::Insert(logical_id, _instance) = component_change.action else {
            panic!("Not the expected change");
        };

        assert_eq!("test", logical_id.as_str());
    }

    #[test]
    fn dont_scale_unloaded_actor() {
        let example_node_id = uuid::Uuid::new_v4();
        let example_node_id_2 = uuid::Uuid::new_v4();

        let (logical_component_id, logical_component_under_test) = crate::ir::actor::mock_actor::MockActorBuilder::default()
            .with_scaling_mode(crate::ir::component::ScalingMode::Scalable {
                min_instances: 1,
                max_instances: 2,
            })
            .build();

        let (physical_component_id_1, physical_component_1) =
            mock_scalable_instance(&logical_component_id, &logical_component_under_test, example_node_id.clone(), 1.0, 2.0);

        let workflow_under_test = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(
                &logical_component_id,
                &logical_component_under_test,
                &[(physical_component_id_1, physical_component_1)],
            )
            .build();

        let mock_node = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id.clone())
            .build()
            .unwrap();

        let mock_node_2 = crate::ir::system_model::mock_node::MockNodeBuilder::default()
            .id(example_node_id_2.clone())
            .build()
            .unwrap();

        let nodes = std::collections::HashMap::from([
            (example_node_id.clone(), &mock_node as &dyn crate::ir::Node),
            (example_node_id_2.clone(), &mock_node_2 as &dyn crate::ir::Node),
        ]);

        let mut scaler = super::Scaler::new();
        let changes = scaler.apply(&workflow_under_test, &nodes, &crate::ir::Clusters::default());

        assert_eq!(changes.len(), 0);
    }

    fn mock_scalable_instance(
        logical_component_id: &str,
        logical_component: &crate::ir::logical_model::LogicalComponent,
        node_id: uuid::Uuid,
        message_rate: f64,
        processing_rate: f64,
    ) -> (uuid::Uuid, crate::ir::PhysicalComponentState) {
        let (_, physical_id, physical_component) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(logical_component_id, logical_component)
                .with_node_id(node_id)
                .with_materialized_actor(crate::ir::actor::MaterializedActor {
                    mapping: crate::ir::MaterializedPorts {
                        materialized_outputs: Default::default(),
                        materialized_inputs: std::collections::HashMap::from([(
                            edgeless_api::function_instance::PortId("a".to_string()),
                            crate::ir::MaterializedInput {
                                mapping: crate::ir::interaction::DestiantionPortMapping {
                                    dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                                        base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                                        constraints: Default::default(),
                                    },
                                    mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlayDestinationPort {
                                        sources: Default::default(),
                                    }),
                                },
                                port_statistics: Some(Box::new(crate::ir::test::MockPortStats::new(
                                    std::collections::HashMap::from([(
                                        edgeless_api::function_instance::InstanceId {
                                            node_id: uuid::Uuid::new_v4(),
                                            function_id: uuid::Uuid::new_v4(),
                                        },
                                        message_rate,
                                    )]),
                                    Default::default(),
                                ))),
                            },
                        )]),
                    },
                    runtime_statistics: Some(Box::new(
                        crate::ir::test::MockRuntimeStatisticsBuilder::default()
                            .invocation_rate(processing_rate)
                            .build()
                            .unwrap(),
                    )),
                })
                .build();

        (physical_id, physical_component)
    }
}
