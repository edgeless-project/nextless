// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Clone, Debug)]
pub struct LogicalActor {
    pub image: super::behavior::Behavior,
    pub annotations: std::collections::HashMap<String, String>,
    pub scaling_mode: crate::ir::component::ScalingMode,
    pub node_filter: crate::ir::component::NodeFilters,
    pub logical_ports: super::LogicalPorts,
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
    pub(crate) materialized: Option<MaterializedActor>,
    pub(crate) annotations: std::collections::HashMap<String, String>,
}

#[derive(Clone)]
pub enum ImageState {
    Planned(super::behavior::BehaviorImageId),
    Existing(super::behavior::BehaviorImage),
}

impl super::PhysicalComponent for PhysicalActor {
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

    // TODO add proper input mapping handling preventing the creation of overlay inputs
    #[tracing::instrument(name = "materialize_actor", skip_all)]
    fn materialize(
        &self,
        telemetry_provider: &Option<Box<dyn super::TelemetryProvider>>,
    ) -> (Vec<crate::ir::transformations::PhysicalChange>, Vec<super::RequiredChange>) {
        let mut material_changes = Vec::new();
        let mut model_changes = Vec::new();
        let mut cloned_self = self.clone();
        if let Some(materialized) = &mut cloned_self.materialized {
            if !materialized.mapping.is_current_mapping(&self.desired_mapping) {
                material_changes.push(super::RequiredChange::PatchFunction {
                    function_id: self.id,
                    function_name: self.component_name.clone(),
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
            material_changes.push(super::RequiredChange::StartFunction {
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

            let mut cloned_self = self.clone();

            cloned_self.materialized = Some(super::actor::MaterializedActor {
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

    fn as_actor(&self) -> Option<&self::PhysicalActor> {
        Some(self)
    }

    fn as_actor_mut(&mut self) -> Option<&mut self::PhysicalActor> {
        Some(self)
    }

    fn stop(&self) -> Vec<super::RequiredChange> {
        vec![super::RequiredChange::StopFunction { function_id: self.id }]
    }

    fn logical_parent(&self) -> String {
        self.component_name.clone()
    }
}

#[derive(Clone)]
pub struct MaterializedActor {
    pub(crate) mapping: super::MaterializedPorts,
    pub(crate) runtime_statistics: Option<Box<dyn super::ComponentRuntimeStatistics>>,
}

impl super::MaterializedComponent for MaterializedActor {
    fn materialized_ports(&self) -> &super::MaterializedPorts {
        &self.mapping
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

#[cfg(test)]
pub(crate) mod mock_actor {
    #[derive(Default)]
    pub struct MockActorBuilder {
        logical_id: Option<String>,
        image: Option<crate::ir::behavior::Behavior>,
        annotations: std::collections::HashMap<String, String>,
        scaling_mode: Option<crate::ir::component::ScalingMode>,
        node_filters: Option<crate::ir::component::NodeFilters>,
        logical_ports: Option<crate::ir::LogicalPorts>,
    }

    pub enum DesiredPhysicalComponentState {
        Requested,
        Planned,
        Materialized,
    }

    pub struct MockActorInstanceBuilder {
        node_id: Option<uuid::Uuid>,
        component_id: Option<uuid::Uuid>,
        image: crate::ir::actor::ImageState,
        behavior_spec: crate::ir::behavior::BehaviorSpec,
        component_name: String,
        creation_time: Option<std::time::Instant>,
        desired_mapping: Option<crate::ir::PhysicalPorts>,
        annotations: std::collections::HashMap<String, String>,
        desired_state: DesiredPhysicalComponentState,
        materialized_state: Option<crate::ir::actor::MaterializedActor>,
        runtime_statistics: Option<Box<dyn crate::ir::ComponentRuntimeStatistics>>,
    }

    impl MockActorInstanceBuilder {
        pub fn new_for_logical(logical_id: &str, logical_component: &crate::ir::LogicalComponent) -> Self {
            let crate::ir::LogicalComponent::Actor(logical_actor) = logical_component else {
                panic!("Bad Component Type");
            };

            Self {
                node_id: None,
                component_id: None,
                image: crate::ir::actor::ImageState::Existing(logical_actor.image.main_image.clone()),
                behavior_spec: logical_actor.image.spec.clone(),
                component_name: logical_id.to_string(),
                creation_time: None,
                desired_mapping: None,
                annotations: logical_actor.annotations.clone(),
                desired_state: DesiredPhysicalComponentState::Materialized,
                materialized_state: None,
                runtime_statistics: None,
            }
        }

        pub fn build(self) -> (String, uuid::Uuid, crate::ir::physical_model::PhysicalComponentState) {
            let id = edgeless_api::function_instance::InstanceId {
                node_id: self.node_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                function_id: self.component_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            };

            let desired_mapping = self.desired_mapping.clone().unwrap_or_default();

            let materialized_state = self.materialized_state.unwrap_or_else(|| crate::ir::actor::MaterializedActor {
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

            let instance = crate::ir::actor::PhysicalActor {
                id: id.clone(),
                component_name: self.component_name.clone(),
                creation_time: self
                    .creation_time
                    .unwrap_or_else(|| std::time::Instant::now() - std::time::Duration::from_secs(60)),
                image: self.image,
                behavior_spec: self.behavior_spec,
                desired_mapping: desired_mapping,
                materialized: match self.desired_state {
                    DesiredPhysicalComponentState::Materialized => Some(materialized_state),
                    DesiredPhysicalComponentState::Planned => None,
                    DesiredPhysicalComponentState::Requested => None,
                },
                annotations: self.annotations,
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

        pub fn with_node_id(mut self, node_id: uuid::Uuid) -> Self {
            self.node_id = Some(node_id);
            self
        }

        #[allow(unused)]
        pub fn with_component_id(mut self, component_id: uuid::Uuid) -> Self {
            self.component_id = Some(component_id);
            self
        }
    }

    impl MockActorBuilder {
        pub fn build(self) -> (String, crate::ir::logical_model::LogicalComponent) {
            let id = self.logical_id.unwrap_or("test".to_string());

            let actor = crate::ir::actor::LogicalActor {
                image: self.image.unwrap_or_else(|| crate::ir::test::mock_actor_image()),
                annotations: self.annotations,
                scaling_mode: self.scaling_mode.unwrap_or_else(|| crate::ir::component::ScalingMode::Singleton),
                node_filter: self.node_filters.unwrap_or_default(),
                logical_ports: self.logical_ports.unwrap_or_default(),
            };

            (id, crate::ir::logical_model::LogicalComponent::Actor(actor))
        }

        pub fn with_logical_id(mut self, id: String) -> Self {
            self.logical_id = Some(id);
            self
        }

        pub fn with_image(mut self, image: crate::ir::behavior::Behavior) -> Self {
            self.image = Some(image);
            self
        }

        #[allow(unused)]
        pub fn with_annotations(mut self, annotations: std::collections::HashMap<String, String>) -> Self {
            self.annotations = annotations;
            self
        }

        #[allow(unused)]
        pub fn with_annotation(mut self, key: &str, val: &str) -> Self {
            self.annotations.insert(key.to_string(), val.to_string());
            self
        }

        pub fn with_scaling_mode(mut self, scaling_mode: crate::ir::component::ScalingMode) -> Self {
            self.scaling_mode = Some(scaling_mode);
            self
        }

        #[allow(unused)]
        pub fn with_node_filters(mut self, node_filters: crate::ir::component::NodeFilters) -> Self {
            self.node_filters = Some(node_filters);
            self
        }

        pub fn with_logical_ports(mut self, logical_ports: crate::ir::LogicalPorts) -> Self {
            self.logical_ports = Some(logical_ports);
            self
        }
    }
}
