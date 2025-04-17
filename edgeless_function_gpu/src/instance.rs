// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
// See lib.rs for more information about the initial source.

extern "C" {
    fn webgpu_instance_new() -> EdgeGpuInstanceId;
    fn webgpu_instance_poll_all(instance_id: EdgeGpuInstanceId, force_wait: u32) -> u32;
    fn webgpu_instance_adapter_create(instance_id: EdgeGpuInstanceId) -> crate::adapter::EdgeGpuAdapterId;
    fn webgpu_instance_wgsl_language_features(instance_id: EdgeGpuInstanceId) -> u32;
}

#[derive(Debug)]
pub struct EdgeGpuInstance {
    ident: EdgeGpuInstanceId,
}

pub(crate) type EdgeGpuInstanceId = u64;

impl Drop for EdgeGpuInstance {
    fn drop(&mut self) {
        unsafe {
            crate::webgpu_drop(self.ident);
        }
    }
}

impl wgpu::custom::InstanceInterface for EdgeGpuInstance {
    fn new(_desc: &wgpu::InstanceDescriptor) -> Self
    where
        Self: Sized,
    {
        let id = unsafe { webgpu_instance_new() };

        Self { ident: id }
    }

    unsafe fn create_surface(&self, _target: wgpu::SurfaceTargetUnsafe) -> Result<wgpu::custom::DispatchSurface, wgpu::CreateSurfaceError> {
        unimplemented!()
    }

    fn request_adapter(&self, _options: &wgpu::RequestAdapterOptions<'_, '_>) -> std::pin::Pin<Box<dyn wgpu::custom::RequestAdapterFuture>> {
        Box::pin(std::future::ready(Ok(wgpu::custom::DispatchAdapter::custom(crate::EdgeGpuAdapter {
            ident: unsafe { webgpu_instance_adapter_create(self.ident) },
        }))))
    }

    fn poll_all_devices(&self, force_wait: bool) -> bool {
        unsafe {
            webgpu_instance_poll_all(
                self.ident,
                match force_wait {
                    true => 1,
                    false => 0,
                },
            ) > 0
        }
    }

    fn wgsl_language_features(&self) -> wgpu::WgslLanguageFeatures {
        let raw = unsafe { webgpu_instance_wgsl_language_features(self.ident) };
        wgpu::WgslLanguageFeatures::from_bits(raw).unwrap()
    }
}
