// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

// use super::MaybePhyiscalInstance;

#[derive(Clone, Debug)]
pub struct LogicalProxy {
    pub logical_ports: super::LogicalPorts,
    pub external_ports: super::ExternalPorts,
}

impl super::LogicalComponentTrait for LogicalProxy {
    fn logical_ports(&self) -> &super::LogicalPorts {
        &self.logical_ports
    }

    fn logical_ports_mut(&mut self) -> &mut super::LogicalPorts {
        &mut self.logical_ports
    }

    fn scaling_mode(&self) -> crate::ir::component::ScalingMode {
        crate::ir::component::ScalingMode::Singleton
    }

    fn node_filters(&self) -> crate::ir::component::NodeFilters {
        crate::ir::component::NodeFilters::default()
    }
}

#[derive(Clone)]
pub struct PhyiscalProxy {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
    pub(crate) external_ports: super::ExternalPorts,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<MaterializedProxy>,
    pub(crate) creation_time: std::time::Instant,
}

impl super::PhysicalComponent for PhyiscalProxy {
    fn physical_ports(&self) -> &super::PhysicalPorts {
        &self.desired_mapping
    }

    fn physical_ports_mut(&mut self) -> &mut super::PhysicalPorts {
        &mut self.desired_mapping
    }

    fn materialized_state(&self) -> Option<&dyn super::MaterializedComponent> {
        self.materialized.as_ref().map(|v| v as &dyn super::MaterializedComponent)
    }

    fn id(&self) -> edgeless_api::function_instance::InstanceId {
        self.id
    }

    fn creation_time(&self) -> std::time::Instant {
        self.creation_time
    }

    fn materialize(
        &self,
        _telemetry_provider: &Option<Box<dyn super::TelemetryProvider>>,
    ) -> (Vec<crate::ir::transformations::PhysicalChange>, Vec<super::RequiredChange>) {
        todo!("Proxy Not Implemented Yet");
    }

    fn stop(&self) -> Vec<super::RequiredChange> {
        vec![super::RequiredChange::StopFunction { function_id: self.id }]
    }

    fn as_actor(&self) -> Option<&super::actor::PhysicalActor> {
        None
    }

    fn as_actor_mut(&mut self) -> Option<&mut super::actor::PhysicalActor> {
        None
    }

    fn logical_parent(&self) -> String {
        todo!("Not Implemented Yet");
    }
}

#[derive(Clone)]
pub struct MaterializedProxy {
    pub(crate) mapping: super::MaterializedPorts,
    pub(crate) runtime_statistics: Option<Box<dyn super::ComponentRuntimeStatistics>>,
}

impl super::MaterializedComponent for MaterializedProxy {
    fn materialized_ports(&self) -> &super::MaterializedPorts {
        &self.mapping
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
                    .map(|i| (edgeless_api::function_instance::PortId(i.id.clone()), i.inner_output.clone().into()))
                    .collect(),
                logical_input_mapping: egress_proxies
                    .iter()
                    .filter_map(|e| match e.inner_input.clone().try_into() {
                        Ok(port) => Some((edgeless_api::function_instance::PortId(e.id.clone()), port)),
                        Err(_) => None,
                    })
                    .collect(),
            },
            external_ports: super::ExternalPorts {
                external_input_mapping: ingress_proxies
                    .iter()
                    .map(|i| {
                        (
                            edgeless_api::function_instance::PortId(i.id.clone()),
                            i.external_input.clone().try_into().unwrap(),
                        )
                    })
                    .collect(),
                external_output_mapping: egress_proxies
                    .iter()
                    .map(|i| (edgeless_api::function_instance::PortId(i.id.clone()), i.external_output.clone().into()))
                    .collect(),
            },
        }
    }
}
