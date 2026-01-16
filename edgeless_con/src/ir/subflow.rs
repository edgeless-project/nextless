// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub struct LogicalSubFlow {
    #[allow(unused)]
    pub(crate) functions: std::collections::HashMap<String, SubFlowFunction>,
    #[allow(unused)]
    pub(crate) resources: std::collections::HashMap<String, SubFlowResource>,

    pub(crate) logical_ports: super::LogicalPorts,

    #[allow(unused)]
    pub(crate) internal_ports: super::InternalPorts,

    pub(crate) instances: Vec<std::cell::RefCell<super::PhysicalComponentState>>,

    #[allow(unused)]
    pub(crate) annotations: std::collections::HashMap<String, String>,
}

impl super::LogicalComponent for LogicalSubFlow {
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
        (&mut self.logical_ports, self.instances.iter().collect())
    }

    fn instances(&self) -> Vec<&std::cell::RefCell<super::PhysicalComponentState>> {
        self.instances.iter().collect()
    }
}

#[derive(Clone)]
pub struct PhysicalSubFlow {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
    #[allow(unused)]
    pub(crate) internal_ports: super::InternalPorts,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<std::cell::RefCell<MaterializedSubflow>>,
    pub(crate) creation_time: std::time::Instant,
}

impl super::PhysicalComponent for PhysicalSubFlow {
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
                changes.push(super::RequiredChange::PatchSubflow {
                    subflow_id: self.id,
                    input_mapping: self.desired_mapping.physical_input_mapping.clone(),
                    output_mapping: self.desired_mapping.physical_output_mapping.clone(),
                });
            }
        } else {
            changes.push(super::RequiredChange::CreateSubflow {
                subflow_id: self.id,
                spawn_req: edgeless_api::workflow_instance::SpawnWorkflowRequest {
                    workflow_functions: Vec::new(),
                    workflow_resources: Vec::new(),
                    workflow_ingress_proxies: Vec::new(),
                    // workflow_ingress_proxies: self
                    //     .desired_mapping
                    //     .physical_input_mapping
                    //     .iter()
                    //     .map(|(id, physical_port)| edgeless_api::workflow_instance::WorkflowIngressProxy {
                    //         id: id.0.clone(),
                    //         inner_output: self.internal_ports.internal_output_mapping.get(id).unwrap().clone(),
                    //         external_input: physical_port.clone(),
                    //     })
                    //     .collect(),
                    workflow_egress_proxies: Vec::new(),
                    // current
                    //     .desired_mapping
                    //     .physical_output_mapping
                    //     .iter()
                    //     .map(|(id, physical_port)| edgeless_api::workflow_instance::WorkflowEgressProxy {
                    //         id: id.0.clone(),
                    //         inner_input: match subflow.logical_ports.logical_input_mapping.get(id).unwrap().clone() {
                    //             super::LogicalInput::Direct(vec) => edgeless_api::workflow_instance::PortMapping::AnyOfTargets(vec),
                    //             super::LogicalInput::Topic(topic) => edgeless_api::workflow_instance::PortMapping::Topic(topic),
                    //         },
                    //         external_output: physical_port.clone(),
                    //     })
                    //     .collect(),
                    annotations: std::collections::HashMap::new(),
                },
            });
        }
        changes
    }

    fn stop(&mut self) -> Vec<super::RequiredChange> {
        vec![super::RequiredChange::StopFunction { function_id: self.id }]
    }

    fn as_actor(&self) -> Option<&super::actor::PhysicalActor> {
        None
    }

    fn as_actor_mut(&mut self) -> Option<&mut super::actor::PhysicalActor> {
        None
    }
}

#[derive(Clone)]
pub struct MaterializedSubflow {
    pub(crate) mapping: super::MaterializedPorts,
    pub(crate) runtime_statistics: Option<Box<dyn super::ComponentRuntimeStatistics>>,
}

impl super::MaterializedComponent for MaterializedSubflow {
    fn materialized_ports(&mut self) -> &mut super::MaterializedPorts {
        &mut self.mapping
    }

    fn runtime_statistics(&self) -> Option<&dyn super::ComponentRuntimeStatistics> {
        self.runtime_statistics.as_deref()
    }
}

pub struct SubFlowFunction {}

pub struct SubFlowResource {}
