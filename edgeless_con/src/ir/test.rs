// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub(crate) struct PhysicalMock {
    id: edgeless_api::function_instance::InstanceId,
    physical_ports: super::PhysicalPorts,
    materialized: std::cell::RefCell<MaterializedMock>,
}

#[derive(Clone)]
pub(crate) struct MaterializedMock {
    materialized_ports: super::MaterializedPorts,
}

#[derive(Clone)]
pub(crate) struct MockPortStats {
    message_rates: std::collections::HashMap<edgeless_api::function_instance::InstanceId, f64>,
    message_sizes: std::collections::HashMap<edgeless_api::function_instance::InstanceId, f64>,
}

pub(crate) struct MockWasmRuntime {}

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
    fn message_rate_abs(&self, _period: std::time::Duration) -> Option<f64> {
        self.message_rates.values().cloned().reduce(|acc, e| acc + e)
    }

    fn message_rate_abs_by_peer(&self, _period: std::time::Duration) -> Vec<(edgeless_api::function_instance::InstanceId, f64)> {
        self.message_rates.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    fn message_size_mean_bytes(&self, _period: std::time::Duration) -> Option<f64> {
        self.message_sizes.values().cloned().reduce(|acc, e| acc + e)
    }

    fn message_size_mean_byte_by_peer(&self, _period: std::time::Duration) -> Vec<(edgeless_api::function_instance::InstanceId, f64)> {
        self.message_sizes.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

impl super::PhysicalComponent for PhysicalMock {
    fn id(&self) -> edgeless_api::function_instance::InstanceId {
        self.id
    }

    fn creation_time(&self) -> std::time::Instant {
        std::time::Instant::now() - std::time::Duration::from_secs(120)
    }

    fn physical_ports(&mut self) -> &mut super::PhysicalPorts {
        &mut self.physical_ports
    }

    fn materialize(&mut self, _telemetry_provider: &Option<Box<dyn super::TelemetryProvider>>) -> Vec<super::RequiredChange> {
        println!("Called Materialize on a PhysicalMock!");
        vec![]
    }

    fn materialized_state(&self) -> Option<&std::cell::RefCell<dyn super::MaterializedComponent>> {
        Some(&self.materialized)
    }

    fn stop(&mut self) -> Vec<super::RequiredChange> {
        println!("Called Stop on a PhysicalMock!");
        vec![]
    }

    fn as_actor_mut(&mut self) -> Option<&mut super::actor::PhysicalActor> {
        None
    }

    fn as_actor(&self) -> Option<&super::actor::PhysicalActor> {
        None
    }
}

impl super::MaterializedComponent for MaterializedMock {
    fn materialized_ports(&mut self) -> &mut super::MaterializedPorts {
        &mut self.materialized_ports
    }

    fn runtime_statistics(&self) -> Option<&dyn super::ComponentRuntimeStatistics> {
        None
    }
}

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

pub(crate) fn component_mock(
    component_id: edgeless_api::function_instance::InstanceId,
    output_1: (edgeless_api::function_instance::InstanceId, f64),
    output_2: (edgeless_api::function_instance::InstanceId, f64),
) -> crate::ir::actor::LogicalActor {
    let cut_logical_ports = crate::ir::LogicalPorts {
        logical_output_mapping: std::collections::HashMap::from([
            (
                edgeless_api::function_instance::PortId("output_1".to_string()),
                crate::ir::interaction::SourcePortMapping {
                    dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                        base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                        constraints: std::collections::BTreeSet::new(),
                    },
                    mapping: Box::new(crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort {
                        destination: crate::ir::interaction::dialect::logical_overlay::DestinationMapping::Unicast(
                            crate::ir::interaction::LogicalPortId {
                                component: "other_1".to_string(),
                                port: edgeless_api::function_instance::PortId("input_1".to_string()),
                            },
                        ),
                    }),
                },
            ),
            (
                edgeless_api::function_instance::PortId("output_2".to_string()),
                crate::ir::interaction::SourcePortMapping {
                    dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                        base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                        constraints: std::collections::BTreeSet::new(),
                    },
                    mapping: Box::new(crate::ir::interaction::dialect::logical_overlay::LogicalOverlaySourcePort {
                        destination: crate::ir::interaction::dialect::logical_overlay::DestinationMapping::Unicast(
                            crate::ir::interaction::LogicalPortId {
                                component: "other_2".to_string(),
                                port: edgeless_api::function_instance::PortId("input_1".to_string()),
                            },
                        ),
                    }),
                },
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
                        mapping: crate::ir::interaction::SourcePortMapping {
                            dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                                base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                                constraints: std::collections::BTreeSet::new(),
                            },
                            mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                                destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(
                                    crate::ir::interaction::PhysicalPortId {
                                        instance: output_1.0.clone(),
                                        port: edgeless_api::function_instance::PortId("input_1".to_string()),
                                    },
                                ),
                            }),
                        },
                        port_statistics: Some(Box::new(crate::ir::test::MockPortStats::new(
                            std::collections::HashMap::from([output_1.clone()]),
                            std::collections::HashMap::new(),
                        ))),
                    },
                ),
                (
                    edgeless_api::function_instance::PortId("output_2".to_string()),
                    crate::ir::MaterializedOutput {
                        mapping: crate::ir::interaction::SourcePortMapping {
                            dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                                base_type: crate::ir::interaction::dialect::physical_overlay::ID,
                                constraints: std::collections::BTreeSet::new(),
                            },
                            mapping: Box::new(crate::ir::interaction::dialect::physical_overlay::PhysicalOverlaySourcePort {
                                destination: crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(
                                    crate::ir::interaction::PhysicalPortId {
                                        instance: output_2.0.clone(),
                                        port: edgeless_api::function_instance::PortId("input_1".to_string()),
                                    },
                                ),
                            }),
                        },
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

pub(crate) fn new_actor_with_mocked_materialized_instances(
    logical_ports: super::LogicalPorts,
    instances: Vec<(edgeless_api::function_instance::InstanceId, super::MaterializedPorts)>,
) -> crate::ir::actor::LogicalActor {
    crate::ir::actor::LogicalActor {
        image: mock_actor_image(),
        annotations: std::collections::HashMap::new(),
        scaling_mode: super::actor::ScalingMode::Scalable {
            min_instances: 1,
            max_instances: 10,
        },
        node_filter: super::actor::NodeFilter::default(),
        logical_ports: logical_ports,
        instances: instances
            .into_iter()
            .map(|(id, materialized_ports)| {
                let instance = PhysicalMock {
                    id,
                    physical_ports: super::PhysicalPorts {
                        physical_output_mapping: materialized_ports
                            .materialized_outputs
                            .iter()
                            .map(|(id, port)| (id.clone(), port.mapping.clone()))
                            .collect(),
                        physical_input_mapping: materialized_ports
                            .materialized_inputs
                            .iter()
                            .map(|(id, port)| (id.clone(), port.mapping.clone()))
                            .collect(),
                    },
                    materialized: std::cell::RefCell::new(MaterializedMock { materialized_ports }),
                };
                std::cell::RefCell::new(crate::ir::PhysicalComponentState::Materialized(Box::new(instance)))
            })
            .collect(),
    }
}

pub(crate) fn mock_nodes_and_candidates<'a>(
    n: usize,
    runtime: &'a dyn crate::ir::WasmRuntime,
    behavior_image_id: crate::ir::behavior::BehaviorImageId,
) -> (Vec<uuid::Uuid>, Vec<crate::ir::transformations::placement::Candidate<'a>>) {
    let mut nodes = Vec::new();
    nodes.resize_with(n, || uuid::Uuid::new_v4());

    let rt = crate::ir::Runtime::WasmBase(runtime, std::collections::BTreeSet::new());

    let candidates: Vec<_> = nodes
        .iter()
        .map(|c| crate::ir::transformations::placement::Candidate {
            node_id: c.clone(),
            runtime: rt.clone(),
            dest_image: crate::ir::actor::ImageState::Planned(crate::ir::behavior::BehaviorImageId {
                behavior_id: behavior_image_id.behavior_id.clone(),
                enabled_ports: behavior_image_id.enabled_ports.clone(),
                dialect_type: rt.supported_dialect(),
            }),
        })
        .collect();

    (nodes, candidates)
}

pub(crate) fn mock_workflow(
    functions: std::collections::HashMap<String, std::cell::RefCell<crate::ir::actor::LogicalActor>>,
) -> crate::ir::workflow::ActiveWorkflow {
    crate::ir::workflow::ActiveWorkflow {
        id: edgeless_api::workflow_instance::WorkflowId {
            workflow_id: uuid::Uuid::new_v4(),
        },
        cluster_id: uuid::Uuid::new_v4(),
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
        feature_flags: crate::ir::workflow::FeatureFlags::default(),
    }
}

pub(crate) fn mock_actor_image() -> crate::ir::behavior::Behavior {
    let behavior_id = crate::ir::behavior::BehaviorId {
        id: "Foo".to_string(),
        version: "0.1".to_string(),
    };

    crate::ir::behavior::Behavior {
        spec: crate::ir::behavior::BehaviorSpec {
            behavior_id: behavior_id.clone(),
            input_ports: std::collections::BTreeMap::new(),
            output_ports: std::collections::BTreeMap::new(),
            inner_structure: std::collections::BTreeMap::new(),
        },
        main_image: crate::ir::behavior::BehaviorImage {
            behavior_image_id: crate::ir::behavior::BehaviorImageId {
                behavior_id: behavior_id,
                enabled_ports: edgeless_api::behavior::EnabledPorts {
                    enabled_inputs: std::collections::BTreeSet::new(),
                    enabled_outputs: std::collections::BTreeSet::new(),
                },
                dialect_type: crate::ir::behavior::dialect::DialectType {
                    base_type: super::behavior::dialect::wasm::ID,
                    features: std::collections::BTreeSet::new(),
                },
            },
            image: Vec::new(),
        },
        extra_images: Vec::new(),
    }
}
