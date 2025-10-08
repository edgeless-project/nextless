// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub trait LogicalComponent {
    fn logical_ports(&self) -> &LogicalPorts;
    fn logical_ports_mut(&mut self) -> &mut LogicalPorts;
    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId>;
    fn instances(&self) -> Vec<&std::cell::RefCell<super::physical_model::PhysicalComponentState>>;
    fn split_view(&mut self) -> (&mut LogicalPorts, Vec<&std::cell::RefCell<super::physical_model::PhysicalComponentState>>);
}

#[derive(Default, Debug)]
pub struct LogicalPorts {
    pub logical_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalOutput>,
    pub logical_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalInput>,
}

#[derive(Clone, Debug)]
pub enum LogicalInput {
    Direct(Vec<(String, edgeless_api::function_instance::PortId)>),
    Topic(String),
}

pub type LogicalOutput = edgeless_api::workflow_instance::PortMapping;
