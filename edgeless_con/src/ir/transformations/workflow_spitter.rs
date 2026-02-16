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

impl super::StatelessLogicalTransformation for WorkflowSplitter {
    #[tracing::instrument(name = "workflow_splitter", skip_all)]
    fn apply(&mut self, _workflow: &crate::ir::workflow::ActiveWorkflow) -> Vec<super::LogicalChange> {
        // TODO
        vec![]
    }
}
