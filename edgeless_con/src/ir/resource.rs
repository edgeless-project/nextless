// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Clone, Debug)]
pub struct LogicalResource {
    pub(crate) class: String,
    pub(crate) configurations: std::collections::HashMap<String, String>,
    pub(crate) logical_ports: super::LogicalPorts,
    pub(crate) scaling_mode: crate::ir::component::ScalingMode,
    pub(crate) node_filters: crate::ir::component::NodeFilters,
}

#[derive(Clone)]
pub struct PhysicalResource {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
    pub(crate) class: String,
    pub(crate) component_name: String,
    pub(crate) configuration: std::collections::HashMap<String, String>,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<MaterializedResource>,
    pub(crate) creation_time: std::time::Instant,
}

impl super::PhysicalComponent for PhysicalResource {
    fn physical_ports(&self) -> &super::PhysicalPorts {
        &self.desired_mapping
    }

    fn physical_ports_mut(&mut self) -> &mut super::PhysicalPorts {
        &mut self.desired_mapping
    }

    fn materialized_state(&self) -> Option<&dyn super::MaterializedComponent> {
        self.materialized.as_ref().map(|v| v as &dyn super::MaterializedComponent)
    }

    fn id(&self) -> edgeless_api::function_instance::InstanceId {
        self.id
    }

    fn creation_time(&self) -> std::time::Instant {
        self.creation_time
    }

    #[tracing::instrument(name = "materialize_resource", skip_all)]
    fn materialize(
        &self,
        telemetry_provider: &Option<Box<dyn super::TelemetryProvider>>,
    ) -> (Vec<crate::ir::transformations::PhysicalChange>, Vec<super::RequiredChange>) {
        let mut material_changes = Vec::new();
        let mut model_changes = Vec::new();
        let mut cloned_self = self.clone();
        if let Some(materialized) = &mut cloned_self.materialized {
            if !materialized.mapping.is_current_mapping(&self.desired_mapping) {
                material_changes.push(super::RequiredChange::PatchResource {
                    resource_id: self.id,
                    resource_name: self.component_name.clone(),
                    input_mapping: self.desired_mapping.physical_input_mapping.clone(),
                    output_mapping: self.desired_mapping.physical_output_mapping.clone(),
                });
                materialized.mapping.update(&self.desired_mapping, &self.id, telemetry_provider);
                model_changes.push(crate::ir::transformations::PhysicalChange::Component(
                    crate::ir::transformations::PhysicalComponentChange {
                        component_id: cloned_self.id.function_id.clone(),
                        // TODO: This should always be in materialized state but we do not verify this here.
                        action: crate::ir::transformations::PhysicalComponentChangeAction::Update(
                            crate::ir::physical_model::PhysicalComponentState::Materialized(Box::new(cloned_self)),
                        ),
                    },
                ));
            }
        } else {
            material_changes.push(super::RequiredChange::StartResource {
                resource_id: self.id,
                resource_name: self.component_name.clone(),
                class_type: self.class.clone(),
                input_mapping: self.desired_mapping.physical_input_mapping.clone(),
                output_mapping: self.desired_mapping.physical_output_mapping.clone(),
                configuration: self.configuration.clone(),
            });

            let mut ports = super::MaterializedPorts::default();
            ports.update(&self.desired_mapping, &self.id, telemetry_provider);

            let mut cloned_self = self.clone();

            cloned_self.materialized = Some(super::resource::MaterializedResource {
                mapping: ports,
                runtime_statistics: telemetry_provider.as_ref().map(|t| t.component_statistics_for(&self.id)),
            });

            model_changes.push(crate::ir::transformations::PhysicalChange::Component(
                crate::ir::transformations::PhysicalComponentChange {
                    component_id: cloned_self.id.function_id.clone(),
                    // We can mark this as materialized here.
                    action: crate::ir::transformations::PhysicalComponentChangeAction::Update(
                        crate::ir::physical_model::PhysicalComponentState::Materialized(Box::new(cloned_self)),
                    ),
                },
            ));
        }
        (model_changes, material_changes)
    }

    fn stop(&self) -> Vec<super::RequiredChange> {
        vec![super::RequiredChange::StopResource { resource_id: self.id }]
    }

    fn as_actor(&self) -> Option<&super::actor::PhysicalActor> {
        None
    }

    fn as_actor_mut(&mut self) -> Option<&mut super::actor::PhysicalActor> {
        None
    }

    fn logical_parent(&self) -> String {
        self.component_name.clone()
    }
}

#[derive(Clone)]
pub struct MaterializedResource {
    pub(crate) mapping: super::MaterializedPorts,
    pub(crate) runtime_statistics: Option<Box<dyn super::ComponentRuntimeStatistics>>,
}

impl super::MaterializedComponent for MaterializedResource {
    fn materialized_ports(&self) -> &super::MaterializedPorts {
        &self.mapping
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
            logical_ports: super::LogicalPorts {
                logical_input_mapping: super::logical_model::parse_api_input_mapping(resource_req.input_mapping),
                logical_output_mapping: super::logical_model::parse_api_output_mapping(resource_req.output_mapping),
            },
            scaling_mode,
            node_filters,
        }
    }
}
