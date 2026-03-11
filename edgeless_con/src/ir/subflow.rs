// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Clone, Debug)]
pub struct LogicalSubFlow {
    #[allow(unused)]
    pub(crate) functions: std::collections::HashMap<String, SubFlowFunction>,
    #[allow(unused)]
    pub(crate) resources: std::collections::HashMap<String, SubFlowResource>,

    pub(crate) logical_ports: super::LogicalPorts,

    #[allow(unused)]
    pub(crate) internal_ports: super::InternalPorts,

    #[allow(unused)]
    pub(crate) annotations: std::collections::HashMap<String, String>,
}

#[derive(Clone)]
pub struct PhysicalSubFlow {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
    #[allow(unused)]
    pub(crate) internal_ports: super::InternalPorts,
    pub(crate) desired_mapping: super::PhysicalPorts,
    pub(crate) materialized: Option<MaterializedSubflow>,
    pub(crate) creation_time: std::time::Instant,
}

impl super::PhysicalComponent for PhysicalSubFlow {
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
        todo!("Subflows Not Implemented Yet");
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

    fn as_resource(&self) -> Option<&super::resource::PhysicalResource> {
        None
    }

    fn as_resource_mut(&mut self) -> Option<&mut super::resource::PhysicalResource> {
        None
    }
}

#[derive(Clone)]
pub struct MaterializedSubflow {
    pub(crate) mapping: super::MaterializedPorts,
    pub(crate) runtime_statistics: Option<Box<dyn super::ComponentRuntimeStatistics>>,
}

impl super::MaterializedComponent for MaterializedSubflow {
    fn materialized_ports(&self) -> &super::MaterializedPorts {
        &self.mapping
    }

    fn runtime_statistics(&self) -> Option<&dyn super::ComponentRuntimeStatistics> {
        self.runtime_statistics.as_deref()
    }
}

#[derive(Clone, Debug)]
pub struct SubFlowFunction {}

#[derive(Clone, Debug)]
pub struct SubFlowResource {}
