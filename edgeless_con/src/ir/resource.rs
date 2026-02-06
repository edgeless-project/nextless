// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub struct LogicalResource {
    pub(crate) class: String,
    pub(crate) configurations: std::collections::HashMap<String, String>,
    pub(crate) instances: Vec<std::cell::RefCell<super::PhysicalComponentState>>,
    pub(crate) logical_ports: super::LogicalPorts,
    pub(crate) scaling_mode: crate::ir::component::ScalingMode,
    pub(crate) node_filters: crate::ir::component::NodeFilters,
}

impl super::LogicalComponent for LogicalResource {
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

    fn instances_mut(&mut self) -> &mut Vec<std::cell::RefCell<super::PhysicalComponentState>> {
        &mut self.instances
    }

    fn scaling_mode(&self) -> crate::ir::component::ScalingMode {
        self.scaling_mode.clone()
    }

    fn node_filters(&self) -> crate::ir::component::NodeFilters {
        self.node_filters.clone()
    }
}

#[derive(Clone)]
pub struct PhysicalResource {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
    pub(crate) class: String,
    pub(crate) component_name: String,
    pub(crate) configuration: std::collections::HashMap<String, String>,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<std::cell::RefCell<MaterializedResource>>,
    pub(crate) creation_time: std::time::Instant,
}

impl super::PhysicalComponent for PhysicalResource {
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

    #[tracing::instrument(name = "materialize_resource", skip_all)]
    fn materialize(&mut self, telemetry_provider: &Option<Box<dyn super::TelemetryProvider>>) -> Vec<super::RequiredChange> {
        let mut changes = Vec::new();
        if let Some(materialized) = &self.materialized {
            let mut materialized = materialized.borrow_mut();
            if !materialized.mapping.is_current_mapping(&self.desired_mapping) {
                changes.push(super::RequiredChange::PatchResource {
                    resource_id: self.id,
                    resource_name: self.component_name.clone(),
                    input_mapping: self.desired_mapping.physical_input_mapping.clone(),
                    output_mapping: self.desired_mapping.physical_output_mapping.clone(),
                });
                materialized.mapping.update(&self.desired_mapping, &self.id, telemetry_provider);
            }
        } else {
            changes.push(super::RequiredChange::StartResource {
                resource_id: self.id,
                resource_name: self.component_name.clone(),
                class_type: self.class.clone(),
                input_mapping: self.desired_mapping.physical_input_mapping.clone(),
                output_mapping: self.desired_mapping.physical_output_mapping.clone(),
                configuration: self.configuration.clone(),
            });

            let mut ports = super::MaterializedPorts::default();
            ports.update(&self.desired_mapping, &self.id, telemetry_provider);

            self.materialized = Some(std::cell::RefCell::new(super::resource::MaterializedResource {
                mapping: ports,
                runtime_statistics: telemetry_provider.as_ref().map(|t| t.component_statistics_for(&self.id)),
            }))
        }
        changes
    }

    fn stop(&mut self) -> Vec<super::RequiredChange> {
        vec![super::RequiredChange::StopFunction { function_id: self.id }]
    }

    fn as_actor(&self) -> Option<&super::actor::PhysicalActor> {
        None
    }

    fn as_actor_mut(&mut self) -> Option<&mut super::actor::PhysicalActor> {
        None
    }
}

#[derive(Clone)]
pub struct MaterializedResource {
    pub(crate) mapping: super::MaterializedPorts,
    pub(crate) runtime_statistics: Option<Box<dyn super::ComponentRuntimeStatistics>>,
}

impl super::MaterializedComponent for MaterializedResource {
    fn materialized_ports(&mut self) -> &mut super::MaterializedPorts {
        &mut self.mapping
    }

    fn runtime_statistics(&self) -> Option<&dyn super::ComponentRuntimeStatistics> {
        self.runtime_statistics.as_deref()
    }
}

impl From<edgeless_api::workflow_instance::WorkflowResource> for LogicalResource {
    fn from(resource_req: edgeless_api::workflow_instance::WorkflowResource) -> Self {
        let scaling_mode = crate::ir::component::ScalingMode::from_annotations(&resource_req.annotations);
        let node_filters = crate::ir::component::NodeFilters::from_annotations(&resource_req.annotations);

        LogicalResource {
            class: resource_req.class_type,
            configurations: resource_req.configurations,
            instances: Vec::new(),
            logical_ports: super::LogicalPorts {
                logical_input_mapping: super::logical_model::parse_api_input_mapping(resource_req.input_mapping),
                logical_output_mapping: super::logical_model::parse_api_output_mapping(resource_req.output_mapping),
            },
            scaling_mode,
            node_filters,
        }
    }
}
