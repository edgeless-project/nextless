// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
pub trait ControllerAPI: Sync + Send + ControllerAPIClone {
    fn workflow_instance_api(&mut self) -> Box<dyn crate::workflow_instance::WorkflowInstanceAPI>;
    fn node_registration_api(&mut self) -> Box<dyn crate::node_registration::NodeRegistrationAPI>;
    fn image_repository(&mut self) -> Box<dyn crate::image_repository::ImageRepositoryAPI>;
}

// https://stackoverflow.com/a/30353928
pub trait ControllerAPIClone {
    fn clone_box(&self) -> Box<dyn ControllerAPI>;
}
impl<T> ControllerAPIClone for T
where
    T: 'static + ControllerAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn ControllerAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn ControllerAPI> {
    fn clone(&self) -> Box<dyn ControllerAPI> {
        self.clone_box()
    }
}
