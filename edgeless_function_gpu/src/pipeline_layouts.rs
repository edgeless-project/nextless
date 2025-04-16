// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
// See lib.rs for more information about the initial source.

#[derive(Debug)]
pub struct EdgeGpuPipelineLayout {
    pub(crate) ident: u64,
}

impl wgpu::custom::PipelineLayoutInterface for EdgeGpuPipelineLayout {}
