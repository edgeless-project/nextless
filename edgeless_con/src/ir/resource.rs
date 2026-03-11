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
    pub(crate) provider: String,
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
                provider_id: self.provider.clone(),
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

    fn as_resource(&self) -> Option<&super::resource::PhysicalResource> {
        Some(self)
    }

    fn as_resource_mut(&mut self) -> Option<&mut super::resource::PhysicalResource> {
        Some(self)
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

#[cfg(test)]
pub(crate) mod mock_resource {
    use crate::ir::actor::mock_actor::DesiredPhysicalComponentState;

    #[derive(Default)]
    pub struct MockResourceBuilder {
        logical_id: Option<String>,
        class: Option<String>,
        configuration: std::collections::HashMap<String, String>,
        scaling_mode: Option<crate::ir::component::ScalingMode>,
        node_filters: Option<crate::ir::component::NodeFilters>,
        logical_ports: Option<crate::ir::LogicalPorts>,
    }

    pub struct MockResourceInstanceBuilder {
        // Those are always set if we base this on a logical instance.
        configuration: std::collections::HashMap<String, String>,
        class: String,
        component_name: String,
        // Those need to defined for the instance
        node_id: Option<uuid::Uuid>,
        component_id: Option<uuid::Uuid>,
        provider: Option<String>,
        creation_time: Option<std::time::Instant>,
        desired_mapping: Option<crate::ir::PhysicalPorts>,
        materialized_state: Option<crate::ir::resource::MaterializedResource>,
        runtime_statistics: Option<Box<dyn crate::ir::ComponentRuntimeStatistics>>,
        // This is a helper field
        desired_state: crate::ir::actor::mock_actor::DesiredPhysicalComponentState,
    }

    // Derived from MockActorBuilder
    impl MockResourceBuilder {
        pub fn build(self) -> (String, crate::ir::logical_model::LogicalComponent) {
            let id = self.logical_id.unwrap_or("test_resource".to_string());

            let resource = crate::ir::resource::LogicalResource {
                class: self.class.unwrap_or("test_resource_class".to_string()),
                configurations: self.configuration,
                scaling_mode: self.scaling_mode.unwrap_or_else(|| crate::ir::component::ScalingMode::Singleton),
                node_filters: self.node_filters.unwrap_or_default(),
                logical_ports: self.logical_ports.unwrap_or_default(),
            };

            (id, crate::ir::logical_model::LogicalComponent::Resource(resource))
        }

        pub fn with_class(mut self, class: &str) -> Self {
            self.class = Some(class.to_string());
            self
        }
    }

    // Derived from MockActorInstanceBuilder
    impl MockResourceInstanceBuilder {
        pub fn new_for_logical(logical_id: &str, logical_component: &crate::ir::LogicalComponent) -> Self {
            let crate::ir::LogicalComponent::Resource(logical_resource) = logical_component else {
                panic!("Bad Component Type");
            };

            Self {
                node_id: None,
                component_id: None,
                component_name: logical_id.to_string(),
                creation_time: None,
                desired_mapping: None,
                desired_state: DesiredPhysicalComponentState::Materialized,
                materialized_state: None,
                runtime_statistics: None,
                configuration: logical_resource.configurations.clone(),
                class: logical_resource.class.clone(),
                provider: None,
            }
        }

        pub fn build(self) -> (String, uuid::Uuid, crate::ir::physical_model::PhysicalComponentState) {
            let id = edgeless_api::function_instance::InstanceId {
                node_id: self.node_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                function_id: self.component_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            };

            let desired_mapping = self.desired_mapping.clone().unwrap_or_default();

            let materialized_state = self.materialized_state.unwrap_or_else(|| crate::ir::resource::MaterializedResource {
                mapping: crate::ir::MaterializedPorts {
                    materialized_outputs: desired_mapping
                        .physical_output_mapping
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                crate::ir::MaterializedOutput {
                                    mapping: v.clone(),
                                    port_statistics: None,
                                },
                            )
                        })
                        .collect(),
                    materialized_inputs: desired_mapping
                        .physical_input_mapping
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                crate::ir::MaterializedInput {
                                    mapping: v.clone(),
                                    port_statistics: None,
                                },
                            )
                        })
                        .collect(),
                },
                runtime_statistics: self.runtime_statistics,
            });

            let instance = crate::ir::resource::PhysicalResource {
                id: id.clone(),
                component_name: self.component_name.clone(),
                creation_time: self
                    .creation_time
                    .unwrap_or_else(|| std::time::Instant::now() - std::time::Duration::from_secs(60)),
                desired_mapping: desired_mapping,
                materialized: match self.desired_state {
                    DesiredPhysicalComponentState::Materialized => Some(materialized_state),
                    DesiredPhysicalComponentState::Planned => None,
                    DesiredPhysicalComponentState::Requested => None,
                },
                class: self.class.clone(),
                provider: self.provider.unwrap_or("default_resource_provider".to_string()),
                configuration: self.configuration.clone(),
            };

            let physical_component = match self.desired_state {
                DesiredPhysicalComponentState::Materialized => crate::ir::physical_model::PhysicalComponentState::Materialized(Box::new(instance)),
                DesiredPhysicalComponentState::Planned => crate::ir::PhysicalComponentState::Planned(Box::new(instance)),
                DesiredPhysicalComponentState::Requested => crate::ir::PhysicalComponentState::Requested(None),
            };

            (self.component_name, id.function_id, physical_component)
        }

        #[allow(unused)]
        pub fn with_state(mut self, state: DesiredPhysicalComponentState) -> Self {
            self.desired_state = state;
            self
        }
    }
}
