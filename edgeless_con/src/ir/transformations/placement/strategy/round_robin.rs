// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct RoundRobin {}

pub struct RoundRobinState {
    node_order: std::collections::LinkedList<edgeless_api::function_instance::NodeId>,
}

impl Default for RoundRobinState {
    fn default() -> Self {
        Self {
            node_order: std::collections::LinkedList::new(),
        }
    }
}

impl super::PlacementStrategy for RoundRobin {
    type GlobalState = RoundRobinState;

    // This is only designed for a small number of nodes (complexity).
    // It also never removed nodes that don't exist anymore during the normal placement logic (this could be done externally).
    // Note that this is only round-robin in the case the nodes are all candidates.
    fn select_candidate<'a, 'b>(
        &'a mut self,
        candidates: Vec<crate::ir::transformations::placement::Candidate<'b>>,
        state: &mut RoundRobinState,
    ) -> Option<crate::ir::transformations::placement::Candidate<'b>> {
        for c in &candidates {
            if state.node_order.iter().find(|i| **i == c.node_id).is_none() {
                state.node_order.push_back(c.node_id.clone());
            }
        }

        while let Some(current) = state.node_order.pop_front() {
            state.node_order.push_back(current.clone());
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
