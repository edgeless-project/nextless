// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub mod dynamic_colocation;
pub mod static_colocation;
#[cfg(test)]
pub(crate) mod test_helpers;

pub trait FilterStrategy: Send + Sync {
    fn filter_candidates<'b>(
        &mut self,
        logical_component_id: String,
        logical_component: &crate::ir::LogicalComponent,
        candidates: Vec<super::Candidate<'b>>,
        workflow: &crate::ir::workflow::ActiveWorkflow,
    ) -> Vec<super::Candidate<'b>>;

    fn new() -> Self;
}
