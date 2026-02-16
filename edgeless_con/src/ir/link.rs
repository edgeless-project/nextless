// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct WorkflowLink {
    #[allow(unused)]
    pub(crate) id: edgeless_api::link::LinkInstanceId,
    pub(crate) class: edgeless_api::link::LinkType,
    pub(crate) materialized: bool,
    // TODO (turn this into a btreemap for eq stability)
    pub(crate) nodes: Vec<(edgeless_api::function_instance::NodeId, edgeless_api::link::LinkProviderId, Vec<u8>, bool)>,
}
