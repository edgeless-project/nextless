// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::{actor::LogicalActor, link::WorkflowLink, proxy::LogicalProxy, resource::LogicalResource, subflow::LogicalSubFlow, *};

pub struct ActiveWorkflow {
    pub(crate) id: edgeless_api::workflow_instance::WorkflowId,
    pub(crate) cluster_id: uuid::Uuid,

    pub(crate) original_request: edgeless_api::workflow_instance::SpawnWorkflowRequest,

    pub(crate) functions: std::collections::HashMap<String, std::cell::RefCell<LogicalActor>>,
    pub(crate) resources: std::collections::HashMap<String, std::cell::RefCell<LogicalResource>>,
    pub(crate) subflows: std::collections::HashMap<String, std::cell::RefCell<LogicalSubFlow>>,
    pub(crate) proxy: std::cell::RefCell<LogicalProxy>,

    pub(crate) links: std::collections::HashMap<edgeless_api::link::LinkInstanceId, WorkflowLink>,

    pub(crate) feature_flags: FeatureFlags,
}

pub struct FeatureFlags {
    pub disable_application_optimization: bool,
    pub disable_actor_optimization: bool,
}

impl ActiveWorkflow {
    pub fn new(
        request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
        id: edgeless_api::workflow_instance::WorkflowId,
        cluster_id: uuid::Uuid,
    ) -> Self {
        let feature_flags = FeatureFlags::from_annotations(&request.annotations);

        ActiveWorkflow {
            // state: WorkflowState::New,
            id,
            cluster_id,
            original_request: request.clone(),
            functions: request
                .workflow_functions
                .into_iter()
                .map(|function_req| (function_req.name.clone(), std::cell::RefCell::new(LogicalActor::from(function_req))))
                .collect(),
            resources: request
                .workflow_resources
                .into_iter()
                .map(|resource_req| (resource_req.name.clone(), std::cell::RefCell::new(LogicalResource::from(resource_req))))
                .collect(),
            links: std::collections::HashMap::new(),
            subflows: std::collections::HashMap::new(),
            proxy: std::cell::RefCell::new(LogicalProxy::new_from_req(
                &request.workflow_ingress_proxies,
                &request.workflow_egress_proxies,
            )),
            feature_flags,
        }
    }

    pub(crate) fn components(&self) -> Vec<(&str, &std::cell::RefCell<dyn LogicalComponent>)> {
        let mut components = self
            .functions
            .iter()
            .map(|(f_id, f)| (f_id.as_str(), f as &std::cell::RefCell<dyn LogicalComponent>))
            .collect::<Vec<_>>();
        components.append(
            &mut self
                .resources
                .iter()
                .map(|(r_id, r)| (r_id.as_str(), r as &std::cell::RefCell<dyn LogicalComponent>))
                .collect::<Vec<_>>(),
        );
        components.append(
            &mut self
                .subflows
                .iter()
                .map(|(s_id, s)| (s_id.as_str(), s as &std::cell::RefCell<dyn LogicalComponent>))
                .collect::<Vec<_>>(),
        );
        components.push(("__proxy", &self.proxy as &std::cell::RefCell<dyn LogicalComponent>));
        components
    }

    pub(crate) fn get_component(&self, component_name: &str) -> Option<&std::cell::RefCell<dyn LogicalComponent>> {
        if let Some(component) = self.functions.get(component_name) {
            Some(component as &std::cell::RefCell<dyn LogicalComponent>)
        } else if let Some(compoenent) = self.resources.get(component_name) {
            Some(compoenent as &std::cell::RefCell<dyn LogicalComponent>)
        } else if component_name == "__proxy" {
            Some(&self.proxy as &std::cell::RefCell<dyn LogicalComponent>)
        } else {
            None
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
