// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
// See lib.rs for more information about the initial source.

#[derive(Debug)]
pub struct EdgeGpuShaderModule {
    pub(crate) ident: u64,
}

impl wgpu::custom::ShaderModuleInterface for EdgeGpuShaderModule {
    fn get_compilation_info(&self) -> std::pin::Pin<Box<dyn wgpu::custom::ShaderCompilationInfoFuture>> {
        Box::pin(std::future::ready(wgpu::CompilationInfo { messages: Vec::new() }))
    }
}
