// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct RoundRobin {}

#[derive(Default)]
pub struct RoundRobinState {
    node_order: std::collections::LinkedList<edgeless_api::function_instance::NodeId>,
}

impl super::PlacementStrategy for RoundRobin {
    type GlobalState = RoundRobinState;

    // This is only designed for a small number of nodes (complexity).
    // It also never removed nodes that don't exist anymore during the normal placement logic (this could be done externally).
    // Note that this is only round-robin in the case the nodes are all candidates.
    fn select_candidate<'b>(
        &mut self,
        candidates: Vec<crate::ir::transformations::placement::Candidate<'b>>,
        state: &mut RoundRobinState,
    ) -> Option<crate::ir::transformations::placement::Candidate<'b>> {
        for c in &candidates {
            if !state.node_order.iter().any(|i| *i == c.node_id) {
                state.node_order.push_back(c.node_id);
            }
        }

        while let Some(current) = state.node_order.pop_front() {
            state.node_order.push_back(current);
            if let Some(c) = candidates.iter().find(|c| c.node_id == current) {
                return Some(c.clone());
            }
        }

        None
    }

    fn new() -> Self {
        RoundRobin {}
    }
}
