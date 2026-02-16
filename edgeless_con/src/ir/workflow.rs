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
    component_instances: std::collections::BTreeMap<uuid::Uuid, super::PhysicalComponentState>,
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
            if let Some(instance) = self.wf.component_instances.get(id) {
                return Some(crate::ir::physical_model::PhysicalInstance {
                    component_id: id.clone(),
                    component: instance,
                });
            } else {
                tracing::warn!("Outdated Instance Link");
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

    pub(crate) fn components_with_instances(&self) -> PhyiscalComponentIterator {
        PhyiscalComponentIterator::new(self)
    }

    pub(crate) fn get_component_with_instances(&self, component_name: &str) -> Option<(&LogicalComponent, InstanceIterator)> {
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
                transformations::PhysicalChange::Link(physical_link_change) => todo!(),
            }
        }
    }

    pub fn apply_logical_component_change(&mut self, change: crate::ir::transformations::LogicalComponentChange) {
        match change.action {
            transformations::LogicalComponentChangeAction::Delete => {
                if let Some((_c, instances)) = self.components.remove(&change.component_id) {
                    for instance_id in instances {
                        self.component_instances.remove(&instance_id);
                        // TODO: Properly Handle this
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
                        // TODO: Proper stop handling
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
                        // TODO: Proper stop handling
                        tracing::warn!("Not updating existing instances.")
                    }
                } else {
                }
            }
        }
    }

    pub fn apply_physical_component_change(&mut self, change: crate::ir::transformations::PhysicalComponentChange) {
        match change.action {
            transformations::PhysicalComponentChangeAction::Delete => {
                tracing::debug!("Delete Instance");
                let instance = self.component_instances.remove(&change.component_id);
                if let Some(instance) = instance {
                    //TODO: Proper handling
                    tracing::warn!("Not removing logical link");
                } else {
                    tracing::warn!("Tried removing unknown component instance");
                }
            }
            transformations::PhysicalComponentChangeAction::Update(physical_component_state) => {
                tracing::debug!("Update Instance");
                if let Some(component) = physical_component_state.logical_component_id() {
                    if let Some((parent, parent_links)) = self.components.get_mut(&component) {
                        parent_links.insert(change.component_id);
                    }
                }
                let old = self.component_instances.insert(change.component_id, physical_component_state);
                //TODO: Error handling
            }
            transformations::PhysicalComponentChangeAction::Insert(logical_component_id, physical_component_state) => {
                tracing::debug!("Insert Instance");
                if let Some((parent, parent_links)) = self.components.get_mut(&logical_component_id) {
                    parent_links.insert(change.component_id);
                }
                let old = self.component_instances.insert(change.component_id, physical_component_state);
                //TODO: Error handling
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
pub(crate) mod test {
    use crate::ir::test::component_mock_basic;

    pub(crate) fn mock_workflow(
        functions: std::collections::HashMap<String, (crate::ir::logical_model::LogicalComponent, std::collections::BTreeSet<uuid::Uuid>)>,
        component_instances: std::collections::BTreeMap<uuid::Uuid, crate::ir::physical_model::PhysicalComponentState>,
    ) -> crate::ir::workflow::ActiveWorkflow {
        crate::ir::workflow::ActiveWorkflow {
            id: edgeless_api::workflow_instance::WorkflowId {
                workflow_id: uuid::Uuid::new_v4(),
            },
            cluster_id: uuid::Uuid::new_v4(),
            original_request: edgeless_api::workflow_instance::SpawnWorkflowRequest {
                workflow_functions: vec![],
                workflow_resources: vec![],
                workflow_ingress_proxies: vec![],
                workflow_egress_proxies: vec![],
                annotations: std::collections::HashMap::new(),
            },
            links: std::collections::HashMap::new(),
            feature_flags: crate::ir::workflow::FeatureFlags::default(),
            components: functions,
            component_instances: component_instances,
        }
    }

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
        let component_id = edgeless_api::function_instance::InstanceId {
            node_id: uuid::Uuid::from_bytes([1; 16]),
            function_id: uuid::Uuid::from_bytes([2; 16]),
        };

        let (component, _instances) = component_mock_basic("test".to_string(), component_id);

        let mut test_wf = mock_workflow(Default::default(), Default::default());

        assert_eq!(test_wf.components_with_instances().count(), 0);

        test_wf.apply_logical_changes(vec![crate::ir::transformations::LogicalChange::Component(
            crate::ir::transformations::LogicalComponentChange {
                component_id: "test".to_string(),
                action: crate::ir::transformations::LogicalComponentChangeAction::Insert(component),
            },
        )]);

        assert_eq!(test_wf.components_with_instances().count(), 1);
    }

    #[test]
    fn logical_component_update() {
        let component_id = edgeless_api::function_instance::InstanceId {
            node_id: uuid::Uuid::from_bytes([1; 16]),
            function_id: uuid::Uuid::from_bytes([2; 16]),
        };

        let (component, _instances) = component_mock_basic("test".to_string(), component_id);

        let components = std::collections::HashMap::from([("test".to_string(), (component.clone(), Default::default()))]);

        let mut test_wf = mock_workflow(components, Default::default());

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
    fn phyiscal_component_insert() {
        let component_id = edgeless_api::function_instance::InstanceId {
            node_id: uuid::Uuid::from_bytes([1; 16]),
            function_id: uuid::Uuid::from_bytes([2; 16]),
        };

        let (component, instances) = component_mock_basic("test".to_string(), component_id);
        let components = std::collections::HashMap::from([("test".to_string(), (component, Default::default()))]);

        let mut test_wf = mock_workflow(components, Default::default());

        // Pre Insert State
        {
            assert_eq!(test_wf.components_with_instances().count(), 1);
            let (_c_id, _logical_component, instances) = test_wf.components_with_instances().next().unwrap();
            assert_eq!(instances.count(), 0);
        }

        test_wf.apply_physical_changes(vec![crate::ir::transformations::PhysicalChange::Component(
            crate::ir::transformations::PhysicalComponentChange {
                component_id: instances[0].0,
                action: crate::ir::transformations::PhysicalComponentChangeAction::Insert("test".to_string(), instances[0].1.clone()),
            },
        )]);

        {
            assert_eq!(test_wf.components_with_instances().count(), 1);

            let (_c_id, _logical_component, instances) = test_wf.components_with_instances().next().unwrap();

            assert_eq!(instances.clone().count(), 1);
        }
    }

    #[test]
    fn phyiscal_component_update() {
        let component_id = edgeless_api::function_instance::InstanceId {
            node_id: uuid::Uuid::from_bytes([1; 16]),
            function_id: uuid::Uuid::from_bytes([2; 16]),
        };

        let (component, instances) = component_mock_basic("test".to_string(), component_id);
        let components = std::collections::HashMap::from([(
            "test".to_string(),
            (component, std::collections::BTreeSet::from([instances[0].0.clone()])),
        )]);

        let mut test_wf = mock_workflow(components, instances.iter().cloned().collect());

        // Pre Update State
        {
            assert_eq!(test_wf.components_with_instances().count(), 1);
            let (_c_id, _logical_component, mut instances) = test_wf.components_with_instances().next().unwrap();
            assert_eq!(instances.clone().count(), 1);
            let updated = instances.next().unwrap();
            assert!(std::matches!(updated.component, crate::ir::PhysicalComponentState::Materialized(_)));
        }

        let new_instance = instances[0].1.mark_lost().unwrap();

        test_wf.apply_physical_changes(vec![crate::ir::transformations::PhysicalChange::Component(
            crate::ir::transformations::PhysicalComponentChange {
                component_id: instances[0].0,
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
        let component_id = edgeless_api::function_instance::InstanceId {
            node_id: uuid::Uuid::from_bytes([1; 16]),
            function_id: uuid::Uuid::from_bytes([2; 16]),
        };

        let (component, instances) = component_mock_basic("test".to_string(), component_id);
        let components = std::collections::HashMap::from([(
            "test".to_string(),
            (component, std::collections::BTreeSet::from([instances[0].0.clone()])),
        )]);

        let mut test_wf = mock_workflow(components, instances.iter().cloned().collect());

        // Pre Update State
        {
            assert_eq!(test_wf.components_with_instances().count(), 1);
            let (_c_id, _logical_component, mut instances) = test_wf.components_with_instances().next().unwrap();
            assert_eq!(instances.clone().count(), 1);
            let updated = instances.next().unwrap();
            assert!(std::matches!(updated.component, crate::ir::PhysicalComponentState::Materialized(_)));
        }

        test_wf.apply_physical_changes(vec![crate::ir::transformations::PhysicalChange::Component(
            crate::ir::transformations::PhysicalComponentChange {
                component_id: instances[0].0,
                action: crate::ir::transformations::PhysicalComponentChangeAction::Delete,
            },
        )]);

        {
            assert_eq!(test_wf.components_with_instances().count(), 1);
            let (_c_id, _logical_component, instances) = test_wf.components_with_instances().next().unwrap();
            assert_eq!(instances.clone().count(), 0);
        }
    }
}
