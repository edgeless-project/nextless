// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::MaybePhyiscalInstance;

pub struct LogicalActor {
    pub image: ActorImage,
    pub annotations: std::collections::HashMap<String, String>,
    pub constraints: ActorConstraints,
    pub logical_ports: super::LogicalPorts,
    pub instances: Vec<std::cell::RefCell<super::PhysicalComponentState<PhysicalActor>>>,
}

pub struct ActorConstraints {
    pub max_instances: Option<usize>,
    pub domain_id_match_any: Option<Vec<edgeless_api::function_instance::NodeId>>,
    pub node_id_match_any: Option<Vec<edgeless_api::function_instance::NodeId>>,
    pub label_match_all: Vec<String>,
    pub resource_match_all: Vec<String>,
}

impl super::LogicalComponent for LogicalActor {
    fn logical_ports(&mut self) -> &mut super::LogicalPorts {
        &mut self.logical_ports
    }

    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId> {
        self.instances.iter().filter_map(|i| i.borrow().id()).collect()
    }

    fn split_view(&mut self) -> (&mut super::LogicalPorts, Vec<&std::cell::RefCell<dyn super::MaybePhyiscalInstance>>) {
        (
            &mut self.logical_ports,
            self.instances
                .iter()
                .map(|i| i as &std::cell::RefCell<dyn super::MaybePhyiscalInstance>)
                .collect(),
        )
    }

    fn instances(&mut self) -> Vec<&std::cell::RefCell<dyn super::MaybePhyiscalInstance>> {
        self.instances
            .iter()
            .map(|i| i as &std::cell::RefCell<dyn super::MaybePhyiscalInstance>)
            .collect()
    }
}

pub struct PhysicalActor {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
    pub(crate) runtime_type: String,
    pub(crate) creation_tine: std::time::Instant,
    pub(crate) image: Option<ActorImage>,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<std::cell::RefCell<MaterializedActor>>,
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
        self.creation_tine
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

#[derive(Clone, Debug)]
pub struct ActorIdentifier {
    pub id: String,
    pub version: String,
}

#[derive(Clone, Debug)]
pub struct ActorClass {
    pub id: ActorIdentifier,
    pub inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::function_instance::Port>,
    pub outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::function_instance::Port>,
    pub inner_structure: std::collections::HashMap<
        edgeless_api::function_instance::MappingNode,
        std::collections::HashSet<edgeless_api::function_instance::MappingNode>,
    >,
}

#[derive(Clone, Debug)]
pub struct ActorImage {
    pub class: ActorClass,
    pub format: String,
    #[allow(unused)]
    pub enabled_inputs: std::collections::HashSet<edgeless_api::function_instance::PortId>,
    #[allow(unused)]
    pub enabled_outputs: std::collections::HashSet<edgeless_api::function_instance::PortId>,
    pub code: Vec<u8>,
}

impl From<edgeless_api::workflow_instance::WorkflowFunction> for LogicalActor {
    fn from(function_req: edgeless_api::workflow_instance::WorkflowFunction) -> Self {
        Self {
            image: ActorImage {
                enabled_inputs: function_req.function_class_specification.function_class_inputs.keys().cloned().collect(),
                enabled_outputs: function_req.function_class_specification.function_class_outputs.keys().cloned().collect(),
                class: ActorClass {
                    id: ActorIdentifier {
                        id: function_req.function_class_specification.function_class_id,
                        version: function_req.function_class_specification.function_class_version,
                    },
                    inputs: function_req.function_class_specification.function_class_inputs,
                    outputs: function_req.function_class_specification.function_class_outputs,
                    inner_structure: function_req
                        .function_class_specification
                        .function_class_inner_structure
                        .into_iter()
                        .map(|(k, v)| (k, std::collections::HashSet::from_iter(v)))
                        .collect(),
                },
                format: function_req.function_class_specification.function_class_type,
                code: function_req.function_class_specification.function_class_code,
            },
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
