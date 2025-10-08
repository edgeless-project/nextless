// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub struct LogicalActor {
    pub image: super::behavior::Behavior,
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
    pub(crate) creation_time: std::time::Instant,
    pub(crate) image: ImageState,
    // Temporary is the Function API still is based on older types
    pub(crate) behavior_spec: super::behavior::BehaviorSpec,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<std::cell::RefCell<MaterializedActor>>,
    pub(crate) annotations: std::collections::HashMap<String, String>,
}

#[derive(Clone)]
pub enum ImageState {
    Planned(super::behavior::BehaviorImageId),
    Existing(super::behavior::BehaviorImage),
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
                image: match &self.image {
                    ImageState::Planned(_) => {
                        // Need proper handling for this
                        panic!("Image Creation Failed");
                    }
                    ImageState::Existing(behavior_image) => behavior_image.clone(),
                },
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

impl From<edgeless_api::workflow_instance::WorkflowFunction> for LogicalActor {
    fn from(function_req: edgeless_api::workflow_instance::WorkflowFunction) -> Self {
        Self {
            image: super::behavior::Behavior::try_from(function_req.behavior).unwrap(),
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
