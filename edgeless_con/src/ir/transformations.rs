// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod colocation_optimizer;
pub mod compiler;
pub mod dead_component_removal;
pub mod logical_interaction_normalizer;
pub mod migration_finalizer;
pub mod physical_interaction_specializer;
pub mod physical_mapper;
pub mod placement;
pub mod scaler;
pub mod workflow_spitter;

// pub trait StatelessLogicalTransformation: Send + Sync {
//     fn apply()
// }

pub trait StatelessPhysicalTransformation: Send + Sync {
    fn apply(
        &mut self,
        workflow: &super::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
    ) -> Vec<PhysicalChange>;
}

pub trait StatefulPhysicalTransformation<G>: Send + Sync {
    fn apply(
        &mut self,
        workflow: &super::workflow::ActiveWorkflow,
        nodes: &crate::ir::Nodes,
        peer_clusters: &crate::ir::Clusters,
        global_state: &G,
    ) -> Vec<PhysicalChange>;
}

pub trait StatelessLogicalTransformation: Send + Sync {
    fn apply(&mut self, workflow: &super::workflow::ActiveWorkflow) -> Vec<LogicalChange>;
}

pub trait StatefulLogicalTransformation<G>: Send + Sync {
    fn apply(&mut self, workflow: &super::workflow::ActiveWorkflow, global_state: &G) -> Vec<LogicalChange>;
}

#[derive(Debug)]
pub enum LogicalChange {
    Component(LogicalComponentChange),
}

#[derive(Debug)]
pub struct LogicalComponentChange {
    pub component_id: String,
    pub action: LogicalComponentChangeAction,
}

#[derive(Debug)]
pub enum LogicalComponentChangeAction {
    Delete,
    Update(super::logical_model::LogicalComponent),
    #[allow(unused)]
    Insert(super::logical_model::LogicalComponent),
}

#[derive(Debug, Clone)]
pub enum PhysicalChange {
    Component(PhysicalComponentChange),
    Link(PhysicalLinkChange),
}

#[derive(Debug, Clone)]
pub struct PhysicalLinkChange {
    pub link_id: edgeless_api::link::LinkInstanceId,
    pub action: PhysicalLinkChangeAction,
}

#[derive(Debug, Clone)]
pub enum PhysicalLinkChangeAction {
    Delete,
    Update(crate::ir::link::WorkflowLink),
    Insert(crate::ir::link::WorkflowLink),
}

#[derive(Debug, Clone)]
pub struct PhysicalComponentChange {
    pub component_id: uuid::Uuid,
    pub action: PhysicalComponentChangeAction,
}

#[derive(Clone)]
pub enum PhysicalComponentChangeAction {
    Delete,
    Update(super::physical_model::PhysicalComponentState),
    Insert(String, super::physical_model::PhysicalComponentState),
}

impl std::fmt::Debug for PhysicalComponentChangeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Delete => write!(f, "Delete"),
            Self::Update(_) => f.debug_tuple("Update").finish(),
            Self::Insert(component, _) => f.debug_tuple("Insert").field(component).finish(),
        }
    }
}
