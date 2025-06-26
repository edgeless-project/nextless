// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub(crate) struct MockWasmRuntime {}

impl crate::ir::WasmRuntime for MockWasmRuntime {
    fn num_cores(&self) -> u32 {
        0
    }

    fn cpu_freq_hz(&self) -> f32 {
        1.0
    }

    fn mem_size_bytes(&self) -> u32 {
        0
    }

    fn runtime_info(&self) -> Option<Box<dyn crate::ir::WasmRuntimeInfo>> {
        None
    }
}

pub(crate) struct MockPortStats {
    message_rates: std::collections::HashMap<edgeless_api::function_instance::InstanceId, f64>,
    message_sizes: std::collections::HashMap<edgeless_api::function_instance::InstanceId, f64>,
}

impl MockPortStats {
    pub(crate) fn new(
        message_rates: std::collections::HashMap<edgeless_api::function_instance::InstanceId, f64>,
        message_sizes: std::collections::HashMap<edgeless_api::function_instance::InstanceId, f64>,
    ) -> Self {
        Self {
            message_rates,
            message_sizes,
        }
    }
}

impl crate::ir::PortStatistics for MockPortStats {
    fn message_rate_abs(&self, period: std::time::Duration) -> Option<f64> {
        self.message_rates.values().cloned().reduce(|acc, e| acc + e)
    }

    fn message_rate_abs_by_peer(&self, period: std::time::Duration) -> Vec<(edgeless_api::function_instance::InstanceId, f64)> {
        self.message_rates.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    fn message_size_mean_bytes(&self, period: std::time::Duration) -> Option<f64> {
        self.message_sizes.values().cloned().reduce(|acc, e| acc + e)
    }

    fn message_size_mean_byte_by_peer(&self, period: std::time::Duration) -> Vec<(edgeless_api::function_instance::InstanceId, f64)> {
        self.message_sizes.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

pub(crate) fn mock_nodes_and_candidates<'a>(
    n: usize,
    runtime: &'a dyn crate::ir::WasmRuntime,
) -> (Vec<uuid::Uuid>, Vec<crate::ir::transformations::placement::Candidate<'a>>) {
    let mut nodes = Vec::new();
    nodes.resize_with(n, || uuid::Uuid::new_v4());

    let candidates: Vec<_> = nodes
        .iter()
        .map(|c| crate::ir::transformations::placement::Candidate {
            node_id: c.clone(),
            runtime: crate::ir::Runtime::WasmBase(runtime),
        })
        .collect();

    (nodes, candidates)
}

pub(crate) fn mock_function_under_test(
    instances: Vec<(edgeless_api::function_instance::InstanceId, Box<dyn crate::ir::PortStatistics>)>,
) -> std::cell::RefCell<crate::ir::actor::LogicalActor> {
    std::cell::RefCell::new(crate::ir::actor::LogicalActor {
        image: mock_actor_image(),
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
                std::cell::RefCell::new(crate::ir::PhysicalComponentState::Existing(crate::ir::actor::PhysicalActor {
                    id: instance_id,
                    runtime_type: "RUST_WASM".to_string(),
                    creation_tine: std::time::Instant::now(),
                    image: None,
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
                }))
            })
            .collect(),
    })
}

pub(crate) fn mock_peer_function(
    instance_node_ids: Vec<edgeless_api::function_instance::InstanceId>,
) -> std::cell::RefCell<crate::ir::actor::LogicalActor> {
    std::cell::RefCell::new(crate::ir::actor::LogicalActor {
        image: mock_actor_image(),
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
                std::cell::RefCell::new(crate::ir::PhysicalComponentState::Existing(crate::ir::actor::PhysicalActor {
                    id: instance_id,
                    runtime_type: "RUST_WASM".to_string(),
                    creation_tine: std::time::Instant::now(),
                    image: None,
                    desired_mapping: crate::ir::PhysicalPorts {
                        physical_output_mapping: std::collections::HashMap::new(),
                        physical_input_mapping: std::collections::HashMap::new(),
                    },
                    materialized: None,
                }))
            })
            .collect(),
    })
}

pub(crate) fn mock_workflow(
    functions: std::collections::HashMap<String, std::cell::RefCell<crate::ir::actor::LogicalActor>>,
) -> crate::ir::workflow::ActiveWorkflow {
    crate::ir::workflow::ActiveWorkflow {
        id: edgeless_api::workflow_instance::WorkflowId {
            workflow_id: uuid::Uuid::new_v4(),
        },
        original_request: edgeless_api::workflow_instance::SpawnWorkflowRequest {
            workflow_functions: vec![],
            workflow_resources: vec![],
            workflow_ingress_proxies: vec![],
            workflow_egress_proxies: vec![],
            annotations: std::collections::HashMap::new(),
        },
        functions: functions,
        resources: std::collections::HashMap::new(),
        subflows: std::collections::HashMap::new(),
        proxy: std::cell::RefCell::new(crate::ir::proxy::LogicalProxy {
            logical_ports: crate::ir::LogicalPorts::default(),
            external_ports: crate::ir::ExternalPorts::default(),
            instances: vec![],
        }),
        links: std::collections::HashMap::new(),
    }
}

pub(crate) fn mock_actor_image() -> crate::ir::actor::ActorImage {
    crate::ir::actor::ActorImage {
        id: crate::ir::actor::ActorImageIdent {
            class_id: crate::ir::actor::ActorClassIdent {
                id: "Foo".to_string(),
                version: "0.1".to_string(),
            },
            format: "RUST_WASM".to_string(),
            enabled_inputs: std::collections::BTreeSet::new(),
            enabled_outputs: std::collections::BTreeSet::new(),
        },
        class: crate::ir::actor::ActorClass {
            id: crate::ir::actor::ActorClassIdent {
                id: "Foo".to_string(),
                version: "0.1".to_string(),
            },
            inputs: std::collections::HashMap::new(),
            outputs: std::collections::HashMap::new(),
            inner_structure: std::collections::HashMap::new(),
        },
        code: Vec::new(),
    }
}
