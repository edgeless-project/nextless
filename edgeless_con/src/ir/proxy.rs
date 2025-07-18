// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

// use super::MaybePhyiscalInstance;

pub struct LogicalProxy {
    pub logical_ports: super::LogicalPorts,

    pub external_ports: super::ExternalPorts,

    pub instances: Vec<std::cell::RefCell<super::PhysicalComponentState>>,
}

impl super::LogicalComponent for LogicalProxy {
    fn logical_ports(&self) -> &super::LogicalPorts {
        &self.logical_ports
    }

    fn logical_ports_mut(&mut self) -> &mut super::LogicalPorts {
        &mut self.logical_ports
    }

    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId> {
        self.instances.iter().filter_map(|i| i.borrow().id()).collect()
    }

    fn split_view(&mut self) -> (&mut super::LogicalPorts, Vec<&std::cell::RefCell<super::PhysicalComponentState>>) {
        (
            &mut self.logical_ports,
            self.instances
                .iter()
                // .map(|i| i as &std::cell::RefCell<dyn super::MaybePhyiscalInstance>)
                .collect(),
        )
    }

    fn instances(&self) -> Vec<&std::cell::RefCell<super::PhysicalComponentState>> {
        self.instances
            .iter()
            .collect()
    }
}

pub struct PhyiscalProxy {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
    pub(crate) external_ports: super::ExternalPorts,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<std::cell::RefCell<MaterializedProxy>>,
    pub(crate) creation_time: std::time::Instant,
}

impl super::PhysicalComponent for PhyiscalProxy {
    fn physical_ports(&mut self) -> &mut super::PhysicalPorts {
        &mut self.desired_mapping
    }

    fn materialized_state(&self) -> Option<&std::cell::RefCell<dyn super::MaterializedComponent>> {
        self.materialized
            .as_ref()
            .map(|v| v as &std::cell::RefCell<dyn super::MaterializedComponent>)
    }

    fn id(&self) -> edgeless_api::function_instance::InstanceId {
        self.id
    }

    fn creation_time(&self) -> std::time::Instant {
        self.creation_time
    }

    fn materialize(&mut self, _telemetry_provider: &Option<Box<dyn super::TelemetryProvider>>) -> Vec<super::RequiredChange> {
        let mut changes = Vec::new();
        if let Some(materialized) = &self.materialized {
            let materialized = materialized.borrow_mut();
            if !materialized.mapping.is_current_mapping(&self.desired_mapping) {
                changes.push(super::RequiredChange::PatchProxy {
                    proxy_id: self.id,
                    internal_inputs: self.desired_mapping.physical_input_mapping.clone(),
                    internal_outputs: self.desired_mapping.physical_output_mapping.clone(),
                    external_inputs: self.external_ports.external_input_mapping.clone(),
                    external_outputs: self.external_ports.external_output_mapping.clone(),
                })
            } else {
                changes.push(super::RequiredChange::CrateProxy {
                    proxy_id: self.id,
                    internal_inputs: self.desired_mapping.physical_input_mapping.clone(),
                    internal_outputs: self.desired_mapping.physical_output_mapping.clone(),
                    external_inputs: self.external_ports.external_input_mapping.clone(),
                    external_outputs: self.external_ports.external_output_mapping.clone(),
                });
            }
        }
        changes
    }

    fn stop(&mut self) -> Vec<super::RequiredChange> {
        vec![super::RequiredChange::StopFunction { function_id: self.id }]
    }

    fn as_actor(&mut self) -> Option<&mut super::actor::PhysicalActor> {
        None
    }
}

pub struct MaterializedProxy {
    pub(crate) mapping: super::MaterializedPorts,
    pub(crate) runtime_statistics: Option<Box<dyn super::ComponentRuntimeStatistics>>,
}

impl super::MaterializedComponent for MaterializedProxy {
    fn materialized_ports(&mut self) -> &mut super::MaterializedPorts {
        &mut self.mapping
    }

    fn runtime_statistics(&self) -> Option<&dyn super::ComponentRuntimeStatistics> {
        self.runtime_statistics.as_deref()
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
