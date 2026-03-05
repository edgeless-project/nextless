// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub(crate) struct MockPortStats {
    message_rates: std::collections::HashMap<edgeless_api::function_instance::InstanceId, f64>,
    message_sizes: std::collections::HashMap<edgeless_api::function_instance::InstanceId, f64>,
}

pub(crate) struct MockWasmRuntime {}

#[derive(Debug, Clone, derive_builder::Builder)]
pub(crate) struct MockRuntimeStatistics {
    #[builder(default)]
    invocation_rate: f64,
    #[builder(default)]
    invocation_rate_by_port: Vec<(edgeless_api::function_instance::PortId, f64)>,
    #[builder(default)]
    duration_mean_secs: f64,
    #[builder(default)]
    duration_soft_limit_rate_rel: f64,
    #[builder(default)]
    error_rate_rel: f64,
}

impl super::ComponentRuntimeStatistics for MockRuntimeStatistics {
    fn invocation_rate_abs(&self, _period: std::time::Duration) -> Option<f64> {
        Some(self.invocation_rate)
    }

    fn invocations_rate_abs_by_port(&self, _period: std::time::Duration) -> Vec<(edgeless_api::function_instance::PortId, f64)> {
        self.invocation_rate_by_port.clone()
    }

    fn duration_mean_secs(&self, _period: std::time::Duration) -> Option<f64> {
        Some(self.duration_mean_secs)
    }

    fn duration_soft_limit_rate_rel(&self, _period: std::time::Duration) -> Option<f64> {
        Some(self.duration_soft_limit_rate_rel)
    }

    fn error_rate_rel(&self, _period: std::time::Duration) -> Option<f64> {
        Some(self.error_rate_rel)
    }
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

pub(crate) fn multi_output_materialized_actor(
    logical_id: String,
    component_id: edgeless_api::function_instance::InstanceId,
    output_1: (edgeless_api::function_instance::InstanceId, f64),
    output_2: (edgeless_api::function_instance::InstanceId, f64),
) -> (
    crate::ir::logical_model::LogicalComponent,
    Vec<(uuid::Uuid, crate::ir::physical_model::PhysicalComponentState)>,
) {
    let output_rates = std::collections::HashMap::from([("output_1", output_1), ("output_2", output_2)]);

    let cut_logical_ports = crate::ir::LogicalPorts {
        logical_output_mapping: std::collections::HashMap::from([
            crate::ir::interaction::dialect::logical_overlay::mock_ports::mock_source_port(
                &edgeless_api::function_instance::PortId("output_1".to_string()),
                crate::ir::interaction::dialect::logical_overlay::DestinationMapping::Unicast(crate::ir::interaction::LogicalPortId {
                    component: "other_1".to_string(),
                    port: edgeless_api::function_instance::PortId("input_1".to_string()),
                }),
            ),
            crate::ir::interaction::dialect::logical_overlay::mock_ports::mock_source_port(
                &edgeless_api::function_instance::PortId("output_2".to_string()),
                crate::ir::interaction::dialect::logical_overlay::DestinationMapping::Unicast(crate::ir::interaction::LogicalPortId {
                    component: "other_2".to_string(),
                    port: edgeless_api::function_instance::PortId("input_1".to_string()),
                }),
            ),
        ]),
        logical_input_mapping: std::collections::HashMap::new(),
    };

    let default_cluster_id = uuid::Uuid::from_u128(1234);

    let output_port_1 = edgeless_api::function_instance::PortId("output_1".to_string());
    let output_port_2 = edgeless_api::function_instance::PortId("output_2".to_string());
    let input_port_1 = edgeless_api::function_instance::PortId("input_1".to_string());
    let instance_1_desired_outputs = std::collections::HashMap::from([
        crate::ir::interaction::dialect::physical_overlay::mock_ports::mock_source_port(
            default_cluster_id,
            &output_port_1,
            crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(crate::ir::interaction::PhysicalPortId {
                instance: output_1.0.clone(),
                port: input_port_1.clone(),
            }),
        ),
        crate::ir::interaction::dialect::physical_overlay::mock_ports::mock_source_port(
            default_cluster_id,
            &output_port_2,
            crate::ir::interaction::dialect::physical_overlay::DestinationMapping::Unicast(crate::ir::interaction::PhysicalPortId {
                instance: output_2.0.clone(),
                port: input_port_1.clone(),
            }),
        ),
    ]);

    let instance_1_materialized_outputs = instance_1_desired_outputs
        .iter()
        .map(|(id, mapping)| {
            (
                id.clone(),
                crate::ir::MaterializedOutput {
                    mapping: mapping.clone(),
                    port_statistics: Some(Box::new(crate::ir::test::MockPortStats::new(
                        std::collections::HashMap::from([output_rates.get(id.0.as_str()).unwrap().clone()]),
                        std::collections::HashMap::new(),
                    ))),
                },
            )
        })
        .collect();

    let instance_1 = (
        component_id,
        crate::ir::PhysicalPorts {
            physical_output_mapping: instance_1_desired_outputs,
            physical_input_mapping: std::collections::HashMap::new(),
        },
        crate::ir::MaterializedPorts {
            materialized_outputs: instance_1_materialized_outputs,
            materialized_inputs: std::collections::HashMap::new(),
        },
    );

    let (logical_id, logical_instance) = crate::ir::actor::mock_actor::MockActorBuilder::default()
        .with_logical_id(logical_id)
        .with_logical_ports(cut_logical_ports)
        .build();

    let mock_instances = vec![instance_1]
        .iter()
        .map(|(instance_id, desired_ports, materialized_ports)| {
            let (_, id, instance) = crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&logical_id, &logical_instance)
                .with_component_id(instance_id.function_id)
                .with_node_id(instance_id.node_id)
                .with_desired_mapping(desired_ports.clone())
                .with_materialized_actor(crate::ir::actor::MaterializedActor {
                    mapping: materialized_ports.clone(),
                    runtime_statistics: None,
                })
                .build();
            (id, instance)
        })
        .collect();

    (logical_instance, mock_instances)
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
        .map(|c| {
            let actor_c = crate::ir::transformations::placement::ActorCandidate {
                node_id: c.clone(),
                runtime: rt.clone(),
                dest_image: crate::ir::actor::ImageState::Planned(crate::ir::behavior::BehaviorImageId {
                    behavior_id: behavior_image_id.behavior_id.clone(),
                    enabled_ports: behavior_image_id.enabled_ports.clone(),
                    dialect_type: rt.supported_dialect(),
                }),
            };
            crate::ir::transformations::placement::Candidate::Actor(actor_c)
        })
        .collect();

    (nodes, candidates)
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
