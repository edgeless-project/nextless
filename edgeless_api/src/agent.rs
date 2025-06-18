// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
pub trait AgentAPI: AgentAPIClone + Sync + Send {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceAPI<edgeless_api_core::instance_id::InstanceId>>;
    fn node_management_api(&mut self) -> Box<dyn crate::node_management::NodeManagementAPI>;
    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn crate::resource_configuration::ResourceConfigurationAPI<edgeless_api_core::instance_id::InstanceId>>;
    fn link_instance_api(&mut self) -> Box<dyn crate::link::LinkInstanceAPI>;
    fn proxy_instance_api(&mut self) -> Box<dyn crate::proxy_instance::ProxyInstanceAPI>;
}

// https://stackoverflow.com/a/30353928
pub trait AgentAPIClone {
    fn clone_box(&self) -> Box<dyn AgentAPI>;
}
impl<T> AgentAPIClone for T
where
    T: 'static + AgentAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn AgentAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn AgentAPI> {
    fn clone(&self) -> Box<dyn AgentAPI> {
        self.clone_box()
    }
}
