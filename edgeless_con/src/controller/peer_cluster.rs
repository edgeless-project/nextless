// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub struct PeerCluster {
    pub _controller_url: String,
    pub api: Box<dyn edgeless_api::controller::ControllerAPI>,
    pub _supported_link_types: std::collections::HashMap<edgeless_api::link::LinkType, edgeless_api::link::LinkProviderId>,
}
