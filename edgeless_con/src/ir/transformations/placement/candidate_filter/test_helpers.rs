// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub(crate) fn mock_function_under_test(
    instances: Vec<(edgeless_api::function_instance::InstanceId, Box<dyn crate::ir::PortStatistics>)>,
) -> std::cell::RefCell<crate::ir::actor::LogicalActor> {
    std::cell::RefCell::new(crate::ir::actor::LogicalActor {
        image: crate::ir::test::mock_actor_image(),
        annotations: std::collections::HashMap::new(),
        constraints: crate::ir::actor::ActorConstraints::default(),
        logical_ports: crate::ir::LogicalPorts {
            logical_output_mapping: std::collections::HashMap::from([(
                edgeless_api::function_instance::PortId("port1".to_string()),
                edgeless_api::workflow_instance::PortMapping::DirectTarget(
                    "f_other".to_string(),
                    edgeless_api::function_instance::PortId("port_other".to_string()),
                ),
            )]),
            logical_input_mapping: std::collections::HashMap::new(),
        },
        instances: instances
            .into_iter()
            .map(|(instance_id, port_statistics)| {
                std::cell::RefCell::new(crate::ir::PhysicalComponentState::Materialized(Box::new(
                    crate::ir::actor::PhysicalActor {
                        id: instance_id,
                        runtime_type: "RUST_WASM".to_string(),
                        creation_time: std::time::Instant::now(),
                        image: crate::ir::test::mock_actor_image(),
                        desired_mapping: crate::ir::PhysicalPorts {
                            physical_output_mapping: std::collections::HashMap::new(),
                            physical_input_mapping: std::collections::HashMap::new(),
                        },
                        materialized: Some(std::cell::RefCell::new(crate::ir::actor::MaterializedActor {
                            mapping: crate::ir::MaterializedPorts {
                                materialized_outputs: std::collections::HashMap::from([(
                                    edgeless_api::function_instance::PortId("port1".to_string()),
                                    crate::ir::MaterializedOutput {
                                        mapping: edgeless_api::common::Output::Single(
                                            edgeless_api::function_instance::InstanceId::new(uuid::Uuid::new_v4()),
                                            edgeless_api::function_instance::PortId("port_other".to_string()),
                                        ),
                                        port_statistics: Some(port_statistics),
                                    },
                                )]),
                                materialized_inputs: std::collections::HashMap::new(),
                            },
                            runtime_statistics: None,
                        })),
                        component_name: "fut".to_string(),
                        annotations: std::collections::HashMap::new(),
                    },
                )))
            })
            .collect(),
    })
}

pub(crate) fn mock_peer_function(
    instance_node_ids: Vec<edgeless_api::function_instance::InstanceId>,
) -> std::cell::RefCell<crate::ir::actor::LogicalActor> {
    std::cell::RefCell::new(crate::ir::actor::LogicalActor {
        image: crate::ir::test::mock_actor_image(),
        annotations: std::collections::HashMap::new(),
        constraints: crate::ir::actor::ActorConstraints::default(),
        logical_ports: crate::ir::LogicalPorts {
            logical_input_mapping: std::collections::HashMap::from([(
                edgeless_api::function_instance::PortId("port_other".to_string()),
                crate::ir::LogicalInput::Direct(vec![("fut".to_string(), edgeless_api::function_instance::PortId("port1".to_string()))]),
            )]),
            logical_output_mapping: std::collections::HashMap::new(),
        },
        instances: instance_node_ids
            .into_iter()
            .map(|instance_id| {
                std::cell::RefCell::new(crate::ir::PhysicalComponentState::Materialized(Box::new(
                    crate::ir::actor::PhysicalActor {
                        id: instance_id,
                        runtime_type: "RUST_WASM".to_string(),
                        creation_time: std::time::Instant::now(),
                        image: crate::ir::test::mock_actor_image(),
                        desired_mapping: crate::ir::PhysicalPorts {
                            physical_output_mapping: std::collections::HashMap::new(),
                            physical_input_mapping: std::collections::HashMap::new(),
                        },
                        materialized: None,
                        component_name: "f_other".to_string(),
                        annotations: std::collections::HashMap::new(),
                    },
                )))
            })
            .collect(),
    })
}
