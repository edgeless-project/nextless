// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub struct WorkflowLink {
    pub(crate) id: edgeless_api::link::LinkInstanceId,
    pub(crate) class: edgeless_api::link::LinkType,
    pub(crate) materialized: bool,
    pub(crate) nodes: Vec<(edgeless_api::function_instance::NodeId, edgeless_api::link::LinkProviderId, Vec<u8>, bool)>,
}
