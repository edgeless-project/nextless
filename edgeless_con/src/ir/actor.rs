// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub struct LogicalActor {
    pub image: super::behavior::Behavior,
    pub annotations: std::collections::HashMap<String, String>,
    pub scaling_mode: crate::ir::component::ScalingMode,
    pub node_filter: crate::ir::component::NodeFilters,
    pub logical_ports: super::LogicalPorts,
    pub instances: Vec<std::cell::RefCell<super::PhysicalComponentState>>,
}

#[derive(Clone)]
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

    // TODO add proper input mapping handling preventing the creation of overlay inputs
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

    fn as_actor(&self) -> Option<&self::PhysicalActor> {
        Some(self)
    }

    fn as_actor_mut(&mut self) -> Option<&mut self::PhysicalActor> {
        Some(self)
    }

    fn stop(&mut self) -> Vec<super::RequiredChange> {
        vec![super::RequiredChange::StopFunction { function_id: self.id }]
    }
}

#[derive(Clone)]
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

impl LogicalActor {
    pub(crate) fn enabled_inputs(&self) -> Vec<edgeless_api::function_instance::PortId> {
        self.logical_ports.logical_input_mapping.iter().map(|i| i.0.clone()).collect()
    }

    pub(crate) fn enabled_outputs(&self) -> Vec<edgeless_api::function_instance::PortId> {
        self.logical_ports.logical_output_mapping.iter().map(|i| i.0.clone()).collect()
    }
}

impl From<edgeless_api::workflow_instance::WorkflowFunction> for LogicalActor {
    fn from(function_req: edgeless_api::workflow_instance::WorkflowFunction) -> Self {
        let scaling_mode = crate::ir::component::ScalingMode::from_annotations(&function_req.annotations);
        let node_filters = crate::ir::component::NodeFilters::from_annotations(&function_req.annotations);
        Self {
            image: super::behavior::Behavior::try_from(function_req.behavior).unwrap(),
            instances: Vec::new(),

            annotations: function_req.annotations,
            logical_ports: super::LogicalPorts {
                logical_input_mapping: super::logical_model::parse_api_input_mapping(function_req.input_mapping),
                logical_output_mapping: super::logical_model::parse_api_output_mapping(function_req.output_mapping),
            },
            scaling_mode,
            node_filter: node_filters,
        }
    }
}
