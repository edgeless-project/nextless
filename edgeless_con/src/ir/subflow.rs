// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::MaybePhyiscalInstance;

pub struct LogicalSubFlow {
    pub(crate) functions: std::collections::HashMap<String, SubFlowFunction>,
    pub(crate) resources: std::collections::HashMap<String, SubFlowResource>,

    pub(crate) logical_ports: super::LogicalPorts,

    pub(crate) internal_ports: super::InternalPorts,

    pub(crate) instances: Vec<std::cell::RefCell<super::PhysicalComponentState<PhysicalSubFlow>>>,

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

    fn split_view(&mut self) -> (&mut super::LogicalPorts, Vec<&std::cell::RefCell<dyn super::MaybePhyiscalInstance>>) {
        (
            &mut self.logical_ports,
            self.instances
                .iter()
                .map(|i| i as &std::cell::RefCell<dyn super::MaybePhyiscalInstance>)
                .collect(),
        )
    }

    fn instances(&self) -> Vec<&std::cell::RefCell<dyn super::MaybePhyiscalInstance>> {
        self.instances
            .iter()
            .map(|i| i as &std::cell::RefCell<dyn super::MaybePhyiscalInstance>)
            .collect()
    }
}

pub struct PhysicalSubFlow {
    pub(crate) id: edgeless_api::function_instance::InstanceId,
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
}

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
