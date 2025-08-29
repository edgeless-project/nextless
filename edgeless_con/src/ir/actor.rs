// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub struct LogicalActor {
    pub image: Behavior,
    pub annotations: std::collections::HashMap<String, String>,
    pub constraints: ActorConstraints,
    pub logical_ports: super::LogicalPorts,
    pub instances: Vec<std::cell::RefCell<super::PhysicalComponentState>>,
}

#[derive(Default)]
pub struct ActorConstraints {
    pub max_instances: Option<usize>,
    pub domain_id_match_any: Option<Vec<edgeless_api::function_instance::NodeId>>,
    pub node_id_match_any: Option<Vec<edgeless_api::function_instance::NodeId>>,
    pub label_match_all: Vec<String>,
    pub resource_match_all: Vec<String>,
}

pub struct PhysicalActor {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
    pub(crate) component_name: String,
    pub(crate) runtime_type: DialectType,
    pub(crate) creation_time: std::time::Instant,
    pub(crate) image: BehaviorImage,
    // Temporary is the Function API still is based on older types
    pub(crate) behavior_spec: BehaviorSpec,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<std::cell::RefCell<MaterializedActor>>,
    pub(crate) annotations: std::collections::HashMap<String, String>,
}

impl super::LogicalComponent for LogicalActor {
    fn logical_ports(&self) -> &super::LogicalPorts {
        &self.logical_ports
    }

    fn logical_ports_mut(&mut self) -> &mut super::LogicalPorts {
        &mut self.logical_ports
    }

    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId> {
        self.instances.iter().filter_map(|i| i.borrow().id()).collect()
    }

    fn split_view(&mut self) -> (&mut super::LogicalPorts, Vec<&std::cell::RefCell<super::PhysicalComponentState>>) {
        (&mut self.logical_ports, self.instances.iter().collect())
    }

    fn instances(&self) -> Vec<&std::cell::RefCell<super::PhysicalComponentState>> {
        self.instances.iter().collect()
    }
}

impl super::PhysicalComponent for PhysicalActor {
    fn physical_ports(&mut self) -> &mut super::PhysicalPorts {
        &mut self.desired_mapping
    }

    fn materialized_state(&self) -> Option<&std::cell::RefCell<dyn super::MaterializedComponent>> {
        self.materialized
            .as_ref()
            .map(|v| v as &std::cell::RefCell<dyn super::MaterializedComponent>)
    }

    fn id(&self) -> edgeless_api::function_instance::InstanceId {
        self.id
    }

    fn creation_time(&self) -> std::time::Instant {
        self.creation_time
    }

    fn materialize(&mut self, telemetry_provider: &Option<Box<dyn super::TelemetryProvider>>) -> Vec<super::RequiredChange> {
        let mut changes = Vec::new();
        if let Some(materialized) = &self.materialized {
            let mut materialized = materialized.borrow_mut();
            if !materialized.mapping.is_current_mapping(&self.desired_mapping) {
                changes.push(super::RequiredChange::PatchFunction {
                    function_id: self.id,
                    function_name: self.component_name.clone(),
                    input_mapping: self.desired_mapping.physical_input_mapping.clone(),
                    output_mapping: self.desired_mapping.physical_output_mapping.clone(),
                });
                materialized.mapping.update(&self.desired_mapping, &self.id, telemetry_provider);
            }
        } else {
            changes.push(super::RequiredChange::StartFunction {
                function_id: self.id,
                function_name: self.component_name.clone(),
                image: self.image.clone(),
                behavior_spec: self.behavior_spec.clone(),
                input_mapping: self.desired_mapping.physical_input_mapping.clone(),
                output_mapping: self.desired_mapping.physical_output_mapping.clone(),
                annotations: self.annotations.clone(),
            });

            let mut ports = super::MaterializedPorts::default();
            ports.update(&self.desired_mapping, &self.id, telemetry_provider);

            self.materialized = Some(std::cell::RefCell::new(super::actor::MaterializedActor {
                mapping: ports,
                runtime_statistics: telemetry_provider.as_ref().map(|t| t.component_statistics_for(&self.id)),
            }))
        }
        changes
    }

    fn as_actor(&mut self) -> Option<&mut self::PhysicalActor> {
        Some(self)
    }

    fn stop(&mut self) -> Vec<super::RequiredChange> {
        vec![super::RequiredChange::StopFunction { function_id: self.id }]
    }
}

pub struct MaterializedActor {
    pub(crate) mapping: super::MaterializedPorts,
    pub(crate) runtime_statistics: Option<Box<dyn super::ComponentRuntimeStatistics>>,
}

impl super::MaterializedComponent for MaterializedActor {
    fn materialized_ports(&mut self) -> &mut super::MaterializedPorts {
        &mut self.mapping
    }

    fn runtime_statistics(&self) -> Option<&dyn super::ComponentRuntimeStatistics> {
        self.runtime_statistics.as_deref()
    }
}

// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
// pub struct ActorClassIdent {
//     pub id: String,
//     pub version: String,
// }
// pub type ActorClassIdent = edgeless_api::behavior::BehaviorId;

// #[derive(Clone, Debug, PartialEq)]
// pub struct ActorClass {
//     pub id: ActorClassIdent,
//     pub inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::function_instance::Port>,
//     pub outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::function_instance::Port>,
//     pub inner_structure: std::collections::HashMap<
//         edgeless_api::function_instance::MappingNode,
//         std::collections::HashSet<edgeless_api::function_instance::MappingNode>,
//     >,
// }

// #[derive(Clone, Debug)]
// pub struct ActorImage {
//     pub id: ActorImageIdent,
//     pub class: ActorClass,
//     pub code: Vec<u8>,
// }

// #[derive(Clone, Debug, PartialEq, Eq, Hash)]
// pub struct ActorImageIdent {
//     pub class_id: ActorClassIdent,
//     pub format: DialectType,
//     pub enabled_inputs: std::collections::BTreeSet<edgeless_api::function_instance::PortId>,
//     pub enabled_outputs: std::collections::BTreeSet<edgeless_api::function_instance::PortId>,
// }

pub type BehaviorId = edgeless_api::behavior::BehaviorId;
pub type BehaviorSpec = edgeless_api::behavior::BehaviorSpec;
pub type EnabledPorts = edgeless_api::behavior::EnabledPorts;

#[derive(Debug, Clone)]
pub struct Behavior {
    pub(crate) spec: BehaviorSpec,
    pub(crate) main_image: BehaviorImage,
    pub(crate) extra_images: Vec<BehaviorImage>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BehaviorImageId {
    pub(crate) behavior_id: BehaviorId,
    pub(crate) enabled_ports: edgeless_api::behavior::EnabledPorts,
    pub(crate) dialect_type: DialectType,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BehaviorImage {
    pub(crate) behavior_image_id: BehaviorImageId,
    pub(crate) image: Vec<u8>,
}

impl TryFrom<edgeless_api::behavior::Behavior> for Behavior {
    type Error = anyhow::Error;

    fn try_from(value: edgeless_api::behavior::Behavior) -> Result<Self, Self::Error> {
        Ok(Self {
            spec: value.spec,
            main_image: value
                .main_image
                .clone()
                .ok_or(anyhow::anyhow!("System currently required a root image"))?
                .try_into()?,
            extra_images: value
                .extra_images
                .into_iter()
                .map(|i| i.try_into())
                .collect::<Result<Vec<_>, Self::Error>>()?,
        })
    }
}

impl TryFrom<edgeless_api::behavior::BehaviorImageId> for BehaviorImageId {
    type Error = anyhow::Error;

    fn try_from(value: edgeless_api::behavior::BehaviorImageId) -> Result<Self, Self::Error> {
        Ok(Self {
            behavior_id: value.behaviour_id,
            enabled_ports: value.enabled_ports,
            dialect_type: value.dialect_type.try_into()?,
        })
    }
}

impl TryFrom<edgeless_api::behavior::BehaviorImage> for BehaviorImage {
    type Error = anyhow::Error;

    fn try_from(value: edgeless_api::behavior::BehaviorImage) -> Result<Self, Self::Error> {
        Ok(BehaviorImage {
            behavior_image_id: value.behavior_image_id.try_into()?,
            image: value.image,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DialectType {
    pub base_type: String,
    pub features: std::collections::BTreeSet<crate::ir::DialectFeature>,
}

impl TryFrom<edgeless_api::node_registration::RuntimeType> for DialectType {
    type Error = anyhow::Error;

    fn try_from(value: edgeless_api::node_registration::RuntimeType) -> Result<Self, Self::Error> {
        Ok(Self {
            features: match value.base_type.as_str() {
                "WASM" => value
                    .features
                    .iter()
                    .map(|f| match f.as_str() {
                        "WGPU" => Ok(crate::ir::DialectFeature::Wasm(crate::ir::WasmRuntimeFeatures::Wgpu)),
                        _ => Err(anyhow::anyhow!("Unknown WASM Feature")),
                    })
                    .collect::<Result<std::collections::BTreeSet<_>, Self::Error>>()?,
                "RUST" => value
                    .features
                    .iter()
                    .map(|f| match f.as_str() {
                        "WGPU" => Ok(crate::ir::DialectFeature::Rust(crate::ir::RustDialectFeatures::Wgpu)),
                        _ => Err(anyhow::anyhow!("Unknown Rust Feature")),
                    })
                    .collect::<Result<std::collections::BTreeSet<_>, Self::Error>>()?,
                _ => return Err(anyhow::anyhow!("Unknown Dialect")),
            },
            base_type: value.base_type,
        })
    }
}

impl From<edgeless_api::workflow_instance::WorkflowFunction> for LogicalActor {
    fn from(function_req: edgeless_api::workflow_instance::WorkflowFunction) -> Self {
        Self {
            image: Behavior::try_from(function_req.behavior).unwrap(),
            // image: ActorImage {
            //     id: ActorImageIdent {
            //         class_id: class_id.clone(),
            //         format: match function_req.function_class_specification.function_class_type.as_str() {
            //             "RUST_BASE" => DialectType {
            //                 base_type: "RUST".to_string(),
            //                 features: std::collections::BTreeSet::new(),
            //             },
            //             "RUST_WGPU" => DialectType {
            //                 base_type: "RUST".to_string(),
            //                 features: std::collections::BTreeSet::from([crate::ir::DialectFeature::Rust(crate::ir::RustDialectFeatures::Wgpu)]),
            //             },
            //             "WASM_BASE" => DialectType {
            //                 base_type: "WASM".to_string(),
            //                 features: std::collections::BTreeSet::new(),
            //             },
            //             "WASM_WGPU" => DialectType {
            //                 base_type: "WASM".to_string(),
            //                 features: std::collections::BTreeSet::from([crate::ir::DialectFeature::Wasm(crate::ir::WasmRuntimeFeatures::Wgpu)]),
            //             },
            //             _ => {
            //                 panic!("Unsupported Type (unhandled)")
            //             }
            //         },
            //         enabled_inputs: function_req.function_class_specification.function_class_inputs.keys().cloned().collect(),
            //         enabled_outputs: function_req.function_class_specification.function_class_outputs.keys().cloned().collect(),
            //     },
            //     class: ActorClass {
            //         id: class_id,
            //         inputs: function_req.function_class_specification.function_class_inputs,
            //         outputs: function_req.function_class_specification.function_class_outputs,
            //         inner_structure: function_req
            //             .function_class_specification
            //             .function_class_inner_structure
            //             .into_iter()
            //             .map(|(k, v)| (k, std::collections::HashSet::from_iter(v)))
            //             .collect(),
            //     },

            //     code: function_req.function_class_specification.function_class_code,
            // },
            instances: Vec::new(),
            constraints: ActorConstraints::from_annotations(&function_req.annotations),
            annotations: function_req.annotations,
            logical_ports: super::LogicalPorts {
                logical_input_mapping: function_req
                    .input_mapping
                    .into_iter()
                    .map(|(port_id, port)| {
                        (
                            port_id,
                            match port {
                                edgeless_api::workflow_instance::PortMapping::DirectTarget(target_fid, target_port) => {
                                    super::LogicalInput::Direct(vec![(target_fid, target_port)])
                                }
                                edgeless_api::workflow_instance::PortMapping::AnyOfTargets(targets) => super::LogicalInput::Direct(targets),
                                edgeless_api::workflow_instance::PortMapping::AllOfTargets(targets) => super::LogicalInput::Direct(targets),
                                edgeless_api::workflow_instance::PortMapping::Topic(topic) => super::LogicalInput::Topic(topic),
                            },
                        )
                    })
                    .collect(),
                logical_output_mapping: function_req.output_mapping.clone(),
            },
        }
    }
}

impl LogicalActor {
    pub(crate) fn enabled_inputs(&self) -> Vec<edgeless_api::function_instance::PortId> {
        self.logical_ports.logical_input_mapping.iter().map(|i| i.0.clone()).collect()
    }

    pub(crate) fn enabled_outputs(&self) -> Vec<edgeless_api::function_instance::PortId> {
        self.logical_ports.logical_output_mapping.iter().map(|i| i.0.clone()).collect()
    }
}

impl ActorConstraints {
    /// Deployment requirements from the annotations in the function's spawn request.
    pub fn from_annotations(annotations: &std::collections::HashMap<String, String>) -> Self {
        let mut max_instances = None;
        if let Some(val) = annotations.get("max_instances") {
            max_instances = Some(val.parse::<usize>().unwrap_or_default());
        }

        let mut node_id_match_any = None;
        if let Some(val) = annotations.get("node_id_match_any") {
            node_id_match_any = Some(val.split(",").filter_map(|x| uuid::Uuid::parse_str(x).ok()).collect());
        }

        let mut domain_id_match_any = None;
        if let Some(val) = annotations.get("domain_id_match_any") {
            domain_id_match_any = Some(val.split(",").filter_map(|x| uuid::Uuid::parse_str(x).ok()).collect());
        }

        let mut label_match_all = vec![];
        if let Some(val) = annotations.get("label_match_all") {
            label_match_all = val.split(",").map(|x| x.to_string()).collect();
        }

        let mut resource_match_all = vec![];
        if let Some(val) = annotations.get("resource_match_all") {
            resource_match_all = val.split(",").map(|x| x.to_string()).collect();
        }

        Self {
            max_instances,
            node_id_match_any,
            label_match_all,
            resource_match_all,
            domain_id_match_any,
        }
    }
}
