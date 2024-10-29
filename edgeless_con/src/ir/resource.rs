// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub struct LogicalResource {
    pub(crate) class: String,
    pub(crate) configurations: std::collections::HashMap<String, String>,

    pub(crate) instances: Vec<std::cell::RefCell<PhysicalResource>>,

    pub(crate) logical_ports: super::LogicalPorts,
}

impl super::LogicalComponent for LogicalResource {
    fn logical_ports(&mut self) -> &mut super::LogicalPorts {
        &mut self.logical_ports
    }

    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId> {
        self.instances.iter().map(|i| i.borrow().id.clone()).collect()
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

pub struct PhysicalResource {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<super::PhysicalPorts>,
}

impl super::PhysicalComponent for PhysicalResource {
    fn physical_ports(&mut self) -> &mut super::PhysicalPorts {
        &mut self.desired_mapping
    }

    fn materialized_state(&mut self) -> &mut dyn super::MaterializedComponent {
        todo!()
    }
}

impl From<edgeless_api::workflow_instance::WorkflowResource> for LogicalResource {
    fn from(resource_req: edgeless_api::workflow_instance::WorkflowResource) -> Self {
        LogicalResource {
            class: resource_req.class_type,
            configurations: resource_req.configurations,
            instances: Vec::new(),
            logical_ports: super::LogicalPorts {
                logical_input_mapping: resource_req
                    .input_mapping
                    .into_iter()
                    .map(|(port_id, port)| {
                        (
                            port_id,
                            match port {
                                edgeless_api::workflow_instance::PortMapping::DirectTarget(target_fid, target_port) => {
                                    super::LogicalInput::Direct(vec![(target_fid, target_port)])
                                }
                                edgeless_api::workflow_instance::PortMapping::AnyOfTargets(targets) => super::LogicalInput::Direct(targets),
                                edgeless_api::workflow_instance::PortMapping::AllOfTargets(targets) => super::LogicalInput::Direct(targets),
                                edgeless_api::workflow_instance::PortMapping::Topic(topic) => super::LogicalInput::Topic(topic),
                            },
                        )
                    })
                    .collect(),
                logical_output_mapping: resource_req.output_mapping.clone(),
            },
        }
    }
}
