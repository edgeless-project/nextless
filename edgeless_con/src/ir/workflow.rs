// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::{actor::LogicalActor, link::WorkflowLink, proxy::LogicalProxy, resource::LogicalResource, *};

pub struct ActiveWorkflow {
    pub(crate) id: edgeless_api::workflow_instance::WorkflowId,
    pub(crate) cluster_id: uuid::Uuid,

    pub(crate) original_request: edgeless_api::workflow_instance::SpawnWorkflowRequest,

    pub(crate) links: std::collections::HashMap<edgeless_api::link::LinkInstanceId, WorkflowLink>,

    pub(crate) feature_flags: FeatureFlags,

    components: std::collections::HashMap<String, (LogicalComponent, std::collections::BTreeSet<uuid::Uuid>)>,
    component_instances: std::collections::BTreeMap<uuid::Uuid, (super::PhysicalComponentState, String)>,
}

pub struct FeatureFlags {
    pub disable_application_optimization: bool,
    pub disable_actor_optimization: bool,
}

#[derive(Clone)]
pub struct PhyiscalComponentIterator<'a> {
    wf: &'a ActiveWorkflow,
    it: std::collections::hash_map::Iter<'a, String, (LogicalComponent, std::collections::BTreeSet<uuid::Uuid>)>,
}

impl<'a> PhyiscalComponentIterator<'a> {
    fn new(workflow: &'a ActiveWorkflow) -> Self {
        Self {
            it: workflow.components.iter(),
            wf: workflow,
        }
    }
}

impl<'a> Iterator for PhyiscalComponentIterator<'a> {
    type Item = (&'a str, &'a LogicalComponent, InstanceIterator<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((component_id, (logicial_component, instance_ids))) = self.it.next() {
            let instance_it = InstanceIterator::new(self.wf, instance_ids.iter());
            return Some((component_id.as_str(), logicial_component, instance_it));
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct InstanceIterator<'a> {
    wf: &'a ActiveWorkflow,
    id_it: std::collections::btree_set::Iter<'a, uuid::Uuid>,
}

impl<'a> InstanceIterator<'a> {
    fn new(wf: &'a ActiveWorkflow, id_it: std::collections::btree_set::Iter<'a, uuid::Uuid>) -> Self {
        Self { wf, id_it }
    }

    pub fn filter_active(self) -> ActiveInstanceIterator<'a> {
        ActiveInstanceIterator { instance_iterator: self }
    }
}

impl<'a> Iterator for InstanceIterator<'a> {
    type Item = crate::ir::physical_model::PhysicalInstance<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(id) = self.id_it.next() {
            if let Some((instance, _)) = self.wf.component_instances.get(id) {
                return Some(crate::ir::physical_model::PhysicalInstance {
                    component_id: id.clone(),
                    component: instance,
                });
            } else {
                tracing::warn!("Outdated Instance Link {id}");
            }
        }

        None
    }
}

pub struct ActiveInstanceIterator<'a> {
    instance_iterator: InstanceIterator<'a>,
}

impl<'a> Iterator for ActiveInstanceIterator<'a> {
    type Item = crate::ir::physical_model::PhysicalInstance<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(instance) = self.instance_iterator.next() {
            if let Some(_active_instance) = instance.component.try_unpack_active() {
                return Some(instance);
            }
        }
        None
    }
}

impl ActiveWorkflow {
    pub fn new(
        request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
        id: edgeless_api::workflow_instance::WorkflowId,
        cluster_id: uuid::Uuid,
    ) -> Self {
        let feature_flags = FeatureFlags::from_annotations(&request.annotations);

        let cloned_request = request.clone();

        let mut components = std::collections::HashMap::new();

        for spawn_actor_req in request.workflow_functions {
            let component_id = spawn_actor_req.name.clone();
            components.insert(
                component_id,
                (LogicalComponent::Actor(LogicalActor::from(spawn_actor_req)), Default::default()),
            );
        }

        for spawn_resource_req in request.workflow_resources {
            let component_id = spawn_resource_req.name.clone();
            components.insert(
                component_id,
                (LogicalComponent::Resource(LogicalResource::from(spawn_resource_req)), Default::default()),
            );
        }

        if !request.workflow_egress_proxies.is_empty() || !request.workflow_ingress_proxies.is_empty() {
            components.insert(
                "__proxy".to_string(),
                (
                    LogicalComponent::Proxy(LogicalProxy::new_from_req(
                        &request.workflow_ingress_proxies,
                        &request.workflow_egress_proxies,
                    )),
                    Default::default(),
                ),
            );
        }

        ActiveWorkflow {
            id,
            cluster_id,
            original_request: cloned_request,
            links: Default::default(),
            components,
            component_instances: Default::default(),
            feature_flags,
        }
    }

    pub(crate) fn components_with_instances<'a>(&'a self) -> PhyiscalComponentIterator<'a> {
        PhyiscalComponentIterator::new(self)
    }

    pub(crate) fn get_component_with_instances<'a>(&'a self, component_name: &str) -> Option<(&'a LogicalComponent, InstanceIterator<'a>)> {
        if let Some((component, component_instance_ids)) = self.components.get(component_name) {
            let instance_iterator = InstanceIterator::new(self, component_instance_ids.iter());
            Some((component, instance_iterator))
        } else {
            None
        }
    }

    pub(crate) fn apply_logical_changes(&mut self, changes: Vec<crate::ir::transformations::LogicalChange>) {
        for change in changes {
            match change {
                transformations::LogicalChange::Component(logical_component_change) => self.apply_logical_component_change(logical_component_change),
            }
        }
    }

    pub(crate) fn apply_physical_changes(&mut self, changes: Vec<crate::ir::transformations::PhysicalChange>) {
        for change in changes {
            match change {
                transformations::PhysicalChange::Component(physical_component_change) => {
                    self.apply_physical_component_change(physical_component_change)
                }
                transformations::PhysicalChange::Link(physical_link_change) => {
                    self.apply_link_change(physical_link_change);
                }
            }
        }
    }

    fn apply_logical_component_change(&mut self, change: crate::ir::transformations::LogicalComponentChange) {
        match change.action {
            transformations::LogicalComponentChangeAction::Delete => {
                if let Some((_c, instances)) = self.components.remove(&change.component_id) {
                    for instance_id in instances {
                        self.component_instances.remove(&instance_id);
                        // TODO: properly handle this
                        tracing::warn!("Dropped component instance without stopping it.");
                    }
                }
            }
            transformations::LogicalComponentChangeAction::Update(logical_component) => {
                let existing_component = self
                    .components
                    .insert(change.component_id.clone(), (logical_component, Default::default()));

                if let Some((_existing_logical_component, existing_instances)) = existing_component {
                    if existing_instances.len() > 0 {
                        // TODO: properly handle this
                        tracing::warn!("Not updating existing instances.")
                    }
                } else {
                    tracing::warn!("Updated component that does not exist");
                }
            }
            transformations::LogicalComponentChangeAction::Insert(logical_component) => {
                let existing_component = self
                    .components
                    .insert(change.component_id.clone(), (logical_component, Default::default()));

                if let Some((_existing_logical_component, existing_instances)) = existing_component {
                    tracing::warn!("Overwriting existing component.");
                    if existing_instances.len() > 0 {
                        // TODO: properly handle this
                        tracing::warn!("Not updating existing instances.")
                    }
                } else {
                    tracing::warn!("Insert replaced existing component.");
                }
            }
        }
    }

    fn apply_physical_component_change(&mut self, change: crate::ir::transformations::PhysicalComponentChange) {
        match change.action {
            transformations::PhysicalComponentChangeAction::Delete => {
                tracing::debug!("Delete Instance");
                let removed = self.component_instances.remove(&change.component_id);
                if let Some((_instance, logical_component_id)) = removed {
                    let found_instance = self
                        .components
                        .get_mut(&logical_component_id)
                        .map(|item| item.1.remove(&change.component_id))
                        .unwrap_or(false);

                    if !found_instance {
                        tracing::warn!("Inconsistent workflow state: Missing instance link for logical component {logical_component_id}");
                    }
                } else {
                    tracing::warn!("Tried to remove unknown component instance.");
                }
            }
            transformations::PhysicalComponentChangeAction::Update(physical_component_state) => {
                tracing::debug!("Update Instance");

                let old = self.component_instances.get_mut(&change.component_id);

                let Some((old_component, old_logical_id)) = old else {
                    tracing::warn!("Physical Component Update on non-existing instance. Request Ignored.");
                    return;
                };

                if let Some((_parent, parent_links)) = self.components.get_mut(old_logical_id.as_str()) {
                    parent_links.insert(change.component_id);
                } else {
                    tracing::warn!("State error: No Logical Component corresponding to the instance.");
                }

                *old_component = physical_component_state;
            }
            transformations::PhysicalComponentChangeAction::Insert(logical_component_id, physical_component_state) => {
                tracing::debug!("Insert Instance");
                if let Some((_parent, parent_links)) = self.components.get_mut(&logical_component_id) {
                    parent_links.insert(change.component_id);
                } else {
                    tracing::warn!("State error: Logical Component does not exist.");
                }
                let old = self
                    .component_instances
                    .insert(change.component_id, (physical_component_state, logical_component_id.clone()));
                if old.is_some() {
                    tracing::warn!("Physical Component Insert replaced existing physical instance.");
                }
            }
        }
    }

    fn apply_link_change(&mut self, change: crate::ir::transformations::PhysicalLinkChange) {
        match change.action {
            transformations::PhysicalLinkChangeAction::Delete => {
                let old = self.links.remove(&change.link_id);

                if old.is_none() {
                    tracing::warn!("Tried to remove link that does not exist");
                }
            }
            transformations::PhysicalLinkChangeAction::Update(workflow_link) => {
                let old = self.links.insert(change.link_id, workflow_link);

                if old.is_none() {
                    tracing::warn!("Link Update inserted new link.");
                }
            }
            transformations::PhysicalLinkChangeAction::Insert(workflow_link) => {
                let old = self.links.insert(change.link_id, workflow_link);

                if old.is_some() {
                    tracing::warn!("Link Update replaced existing link.");
                }
            }
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            disable_application_optimization: false,
            disable_actor_optimization: false,
        }
    }
}

impl FeatureFlags {
    fn from_annotations(annotations: &std::collections::HashMap<String, String>) -> Self {
        let mut flags = Self::default();

        if let Some(string_flags) = annotations.get("feature_flags") {
            for flag in string_flags.split(",") {
                match flag {
                    "disable_application_optimization" => flags.disable_application_optimization = true,
                    "disable_actor_optimization" => flags.disable_actor_optimization = true,
                    _ => {}
                }
            }
        }

        flags
    }
}

#[cfg(test)]
pub(crate) mod mock_workflow {
    #[derive(Default)]
    pub struct MockWorkflowBuilder {
        id: Option<uuid::Uuid>,
        cluster_id: Option<uuid::Uuid>,
        links: std::collections::HashMap<edgeless_api::link::LinkInstanceId, crate::ir::link::WorkflowLink>,
        components: std::collections::HashMap<String, (crate::ir::LogicalComponent, Vec<(uuid::Uuid, crate::ir::PhysicalComponentState)>)>,
    }

    impl MockWorkflowBuilder {
        pub fn with_component(
            mut self,
            id: &str,
            logical: &crate::ir::LogicalComponent,
            phyiscal_components: &[(uuid::Uuid, crate::ir::PhysicalComponentState)],
        ) -> Self {
            self.components.insert(id.to_string(), (logical.clone(), phyiscal_components.to_vec()));
            self
        }

        pub fn build(self) -> crate::ir::workflow::ActiveWorkflow {
            let id = edgeless_api::workflow_instance::WorkflowId {
                workflow_id: self.id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            };

            let components = self
                .components
                .iter()
                .map(|(id, (component, instances))| (id.clone(), (component.clone(), instances.iter().map(|(id, _)| id.clone()).collect())))
                .collect();

            let component_instances = self
                .components
                .iter()
                .flat_map(|(logical_id, (_, instances))| {
                    instances
                        .iter()
                        .map(|(id, instance)| (id.clone(), (instance.clone(), logical_id.clone())))
                })
                .collect();

            let workfow = crate::ir::workflow::ActiveWorkflow {
                id,
                cluster_id: self.cluster_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                original_request: edgeless_api::workflow_instance::SpawnWorkflowRequest {
                    workflow_functions: vec![],
                    workflow_resources: vec![],
                    workflow_ingress_proxies: vec![],
                    workflow_egress_proxies: vec![],
                    annotations: std::collections::HashMap::new(),
                },
                links: self.links,
                feature_flags: crate::ir::workflow::FeatureFlags::default(),
                components,
                component_instances,
            };

            workfow
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    #[test]
    fn parse_request() {
        let behavior_id = edgeless_api::behavior::BehaviorId {
            id: "behavior".to_string(),
            version: "0.1".to_string(),
        };

        let request = edgeless_api::workflow_instance::SpawnWorkflowRequest {
            workflow_functions: vec![edgeless_api::workflow_instance::WorkflowFunction {
                name: "a".to_string(),
                behavior: edgeless_api::behavior::Behavior {
                    spec: edgeless_api::behavior::BehaviorSpec {
                        behavior_id: behavior_id.clone(),
                        input_ports: Default::default(),
                        output_ports: Default::default(),
                        inner_structure: Default::default(),
                    },
                    main_image: Some(edgeless_api::behavior::BehaviorImage {
                        behavior_image_id: edgeless_api::behavior::BehaviorImageId {
                            behaviour_id: behavior_id.clone(),
                            enabled_ports: edgeless_api::behavior::EnabledPorts {
                                enabled_inputs: Default::default(),
                                enabled_outputs: Default::default(),
                            },
                            dialect_type: edgeless_api::node_registration::RuntimeType {
                                base_type: "WASM".to_string(),
                                features: Default::default(),
                            },
                        },
                        image: vec![],
                    }),
                    extra_images: vec![],
                },
                output_mapping: Default::default(),
                input_mapping: Default::default(),
                annotations: Default::default(),
            }],
            workflow_resources: Default::default(),
            workflow_ingress_proxies: Default::default(),
            workflow_egress_proxies: Default::default(),
            annotations: Default::default(),
        };

        let wf = super::ActiveWorkflow::new(
            request,
            edgeless_api::workflow_instance::WorkflowId {
                workflow_id: uuid::Uuid::new_v4(),
            },
            uuid::Uuid::new_v4(),
        );

        assert_eq!(wf.components_with_instances().count(), 1);
    }

    #[test]
    fn logical_component_insert() {
        let (component_id, component) = crate::ir::actor::mock_actor::MockActorBuilder::default().build();

        let mut test_wf = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default().build();

        assert_eq!(test_wf.components_with_instances().count(), 0);

        test_wf.apply_logical_changes(vec![crate::ir::transformations::LogicalChange::Component(
            crate::ir::transformations::LogicalComponentChange {
                component_id: component_id.clone(),
                action: crate::ir::transformations::LogicalComponentChangeAction::Insert(component),
            },
        )]);

        assert_eq!(test_wf.components_with_instances().count(), 1);
        assert_eq!(test_wf.components_with_instances().next().unwrap().0, component_id);
    }

    #[test]
    fn logical_component_update() {
        let (component_id, component) = crate::ir::actor::mock_actor::MockActorBuilder::default().build();

        let mut test_wf = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&component_id, &component, &[])
            .build();

        assert_eq!(test_wf.components_with_instances().count(), 1);
        let (_, before_component, _) = test_wf.components_with_instances().next().unwrap();
        assert_eq!(before_component.logical_ports().logical_input_mapping.len(), 0);

        let mut cloned_component = component.clone();
        cloned_component.logical_ports_mut().logical_input_mapping.insert(
            edgeless_api::function_instance::PortId("foo".to_string()),
            crate::ir::interaction::DestiantionPortMapping {
                dialect_type: crate::ir::interaction::dialect::DialectDescriptor {
                    base_type: crate::ir::interaction::dialect::logical_overlay::ID,
                    constraints: Default::default(),
                },
                mapping: Box::new(crate::ir::interaction::dialect::topic_pub_sub::TopicPubSubDestinationPort { filter: "asdf".to_string() }),
            },
        );

        test_wf.apply_logical_changes(vec![crate::ir::transformations::LogicalChange::Component(
            crate::ir::transformations::LogicalComponentChange {
                component_id: "test".to_string(),
                action: crate::ir::transformations::LogicalComponentChangeAction::Insert(cloned_component),
            },
        )]);

        assert_eq!(test_wf.components_with_instances().count(), 1);
        let (_, after_component, _) = test_wf.components_with_instances().next().unwrap();
        assert_eq!(after_component.logical_ports().logical_input_mapping.len(), 1);
    }

    #[test]
    fn logical_component_delete() {
        let (component_id, component) = crate::ir::actor::mock_actor::MockActorBuilder::default().build();

        let (_, component_instance_id, component_instance) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&component_id, &component).build();

        let mut test_wf = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&component_id, &component, &[(component_instance_id, component_instance.clone())])
            .build();

        // Pre Update State
        {
            assert_eq!(test_wf.components_with_instances().count(), 1);
            let (_c_id, _logical_component, instances) = test_wf.components_with_instances().next().unwrap();
            assert_eq!(instances.clone().count(), 1);
            assert_eq!(test_wf.component_instances.len(), 1);
        }

        test_wf.apply_logical_changes(vec![crate::ir::transformations::LogicalChange::Component(
            crate::ir::transformations::LogicalComponentChange {
                component_id: component_id.clone(),
                action: crate::ir::transformations::LogicalComponentChangeAction::Delete,
            },
        )]);

        // Post Update State
        {
            assert_eq!(test_wf.components_with_instances().count(), 0);
            assert_eq!(test_wf.component_instances.len(), 0);
        }
    }

    #[test]
    fn phyiscal_component_insert() {
        let (component_id, component) = crate::ir::actor::mock_actor::MockActorBuilder::default().build();

        let mut test_wf = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&component_id, &component, &[])
            .build();

        let (_, component_instance_id, component_instance) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&component_id, &component).build();

        // Pre Insert State
        {
            assert_eq!(test_wf.components_with_instances().count(), 1);
            let (_c_id, _logical_component, instances) = test_wf.components_with_instances().next().unwrap();
            assert_eq!(instances.count(), 0);
        }

        test_wf.apply_physical_changes(vec![crate::ir::transformations::PhysicalChange::Component(
            crate::ir::transformations::PhysicalComponentChange {
                component_id: component_instance_id.clone(),
                action: crate::ir::transformations::PhysicalComponentChangeAction::Insert("test".to_string(), component_instance.clone()),
            },
        )]);

        {
            assert_eq!(test_wf.components_with_instances().count(), 1);
            let (_c_id, _logical_component, instances) = test_wf.components_with_instances().next().unwrap();
            assert_eq!(instances.clone().count(), 1);
            assert_eq!(instances.clone().next().unwrap().component_id, component_instance_id);
        }
    }

    #[test]
    fn phyiscal_component_update() {
        let (component_id, component) = crate::ir::actor::mock_actor::MockActorBuilder::default().build();

        let (_, component_instance_id, component_instance) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&component_id, &component).build();

        let mut test_wf = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&component_id, &component, &[(component_instance_id, component_instance.clone())])
            .build();

        // Pre Update State
        {
            assert_eq!(test_wf.components_with_instances().count(), 1);
            let (_c_id, _logical_component, mut instances) = test_wf.components_with_instances().next().unwrap();
            assert_eq!(instances.clone().count(), 1);
            let updated = instances.next().unwrap();
            assert!(std::matches!(updated.component, crate::ir::PhysicalComponentState::Materialized(_)));
        }

        let new_instance = component_instance.mark_lost().unwrap();

        test_wf.apply_physical_changes(vec![crate::ir::transformations::PhysicalChange::Component(
            crate::ir::transformations::PhysicalComponentChange {
                component_id: component_instance_id,
                action: crate::ir::transformations::PhysicalComponentChangeAction::Update(new_instance),
            },
        )]);

        {
            assert_eq!(test_wf.components_with_instances().count(), 1);
            let (_c_id, _logical_component, mut instances) = test_wf.components_with_instances().next().unwrap();
            assert_eq!(instances.clone().count(), 1);
            let updated = instances.next().unwrap();
            assert!(std::matches!(updated.component, crate::ir::PhysicalComponentState::Lost(_)))
        }
    }

    #[test]
    fn phyiscal_component_delete() {
        let (component_id, component) = crate::ir::actor::mock_actor::MockActorBuilder::default().build();

        let (_, component_instance_id, component_instance) =
            crate::ir::actor::mock_actor::MockActorInstanceBuilder::new_for_logical(&component_id, &component).build();

        let mut test_wf = crate::ir::workflow::mock_workflow::MockWorkflowBuilder::default()
            .with_component(&component_id, &component, &[(component_instance_id, component_instance.clone())])
            .build();

        // Pre Update State
        {
            assert_eq!(test_wf.components_with_instances().count(), 1);
            let (_c_id, _logical_component, instances) = test_wf.components_with_instances().next().unwrap();
            assert_eq!(instances.clone().count(), 1);
            // Ensure internal state is consistent
            assert_eq!(test_wf.components.get(&component_id).unwrap().1.len(), 1);
        }

        test_wf.apply_physical_changes(vec![crate::ir::transformations::PhysicalChange::Component(
            crate::ir::transformations::PhysicalComponentChange {
                component_id: component_instance_id,
                action: crate::ir::transformations::PhysicalComponentChangeAction::Delete,
            },
        )]);

        {
            assert_eq!(test_wf.components_with_instances().count(), 1);
            let (_c_id, _logical_component, instances) = test_wf.components_with_instances().next().unwrap();
            assert_eq!(instances.clone().count(), 0);
            // Ensure internal state is consistent
            assert_eq!(test_wf.components.get(&component_id).unwrap().1.len(), 0);
        }
    }
}
