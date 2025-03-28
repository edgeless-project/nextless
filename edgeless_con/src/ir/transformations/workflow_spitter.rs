// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub struct WorkflowSplitter {}

impl WorkflowSplitter {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::StatelessTransformation for WorkflowSplitter {
    fn apply(&mut self, _workflow: &mut crate::ir::workflow::ActiveWorkflow, _nodes: &crate::ir::Nodes, _peer_clusters: &crate::ir::Clusters) {
        // TODO
    }
}
