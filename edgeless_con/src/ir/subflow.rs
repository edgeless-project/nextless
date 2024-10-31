// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub struct LogicalSubFlow {
    pub(crate) functions: std::collections::HashMap<String, SubFlowFunction>,
    pub(crate) resources: std::collections::HashMap<String, SubFlowResource>,

    pub(crate) logical_ports: super::LogicalPorts,

    pub(crate) internal_ports: super::InternalPorts,

    pub(crate) instances: Vec<std::cell::RefCell<PhysicalSubFlow>>,

    pub(crate) annotations: std::collections::HashMap<String, String>,
}

impl super::LogicalComponent for LogicalSubFlow {
    fn logical_ports(&mut self) -> &mut super::LogicalPorts {
        &mut self.logical_ports
    }

    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId> {
        self.instances.iter().map(|i| i.borrow().id).collect()
    }

    fn instances(&mut self) -> Vec<&std::cell::RefCell<dyn super::PhysicalComponent>> {
        self.instances
            .iter()
            .map(|i| i as &std::cell::RefCell<dyn super::PhysicalComponent>)
            .collect()
    }

    fn split_view(&mut self) -> (&mut super::LogicalPorts, Vec<&std::cell::RefCell<dyn super::PhysicalComponent>>) {
        (
            &mut self.logical_ports,
            self.instances
                .iter()
                .map(|i| i as &std::cell::RefCell<dyn super::PhysicalComponent>)
                .collect(),
        )
    }
}

pub struct PhysicalSubFlow {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<super::PhysicalPorts>,
}

impl super::PhysicalComponent for PhysicalSubFlow {
    fn physical_ports(&mut self) -> &mut super::PhysicalPorts {
        &mut self.desired_mapping
    }

    fn materialized_state(&mut self) -> &mut dyn super::MaterializedComponent {
        todo!()
    }
}

pub struct SubFlowFunction {}

pub struct SubFlowResource {}
