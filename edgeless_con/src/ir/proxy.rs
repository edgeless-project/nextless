// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub struct LogicalProxy {
    pub logical_ports: super::LogicalPorts,

    pub external_ports: super::ExternalPorts,

    pub instances: Vec<std::cell::RefCell<PhyiscalProxy>>,
}

impl super::LogicalComponent for LogicalProxy {
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

pub struct PhyiscalProxy {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<super::PhysicalPorts>,
}

impl super::PhysicalComponent for PhyiscalProxy {
    fn physical_ports(&mut self) -> &mut super::PhysicalPorts {
        &mut self.desired_mapping
    }

    fn materialized_state(&mut self) -> &mut dyn super::MaterializedComponent {
        todo!()
    }
}

impl LogicalProxy {
    pub fn new_from_req(
        ingress_proxies: &[edgeless_api::workflow_instance::WorkflowIngressProxy],
        egress_proxies: &[edgeless_api::workflow_instance::WorkflowEgressProxy],
    ) -> Self {
        LogicalProxy {
            logical_ports: super::LogicalPorts {
                logical_output_mapping: ingress_proxies
                    .iter()
                    .map(|i| (edgeless_api::function_instance::PortId(i.id.clone()), i.inner_output.clone()))
                    .collect(),
                logical_input_mapping: egress_proxies
                    .iter()
                    .filter_map(|e| match &e.inner_input {
                        edgeless_api::workflow_instance::PortMapping::Topic(t) => Some((
                            edgeless_api::function_instance::PortId(e.id.clone()),
                            super::LogicalInput::Topic(t.clone()),
                        )),
                        _ => None,
                    })
                    .collect(),
            },
            external_ports: super::ExternalPorts {
                external_input_mapping: ingress_proxies
                    .iter()
                    .map(|i| (edgeless_api::function_instance::PortId(i.id.clone()), i.external_input.clone()))
                    .collect(),
                external_output_mapping: egress_proxies
                    .iter()
                    .map(|i| (edgeless_api::function_instance::PortId(i.id.clone()), i.external_output.clone()))
                    .collect(),
            },
            instances: Vec::new(),
        }
    }
}
