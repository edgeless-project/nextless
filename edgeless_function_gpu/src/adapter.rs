// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
// See lib.rs for more information about the initial source.

extern "C" {
    fn webgpu_adapter_device_create(EdgeGpuAdapterId: u64, out_device_id: *mut u64, out_queue_id: *mut u64) -> u32;
}

#[derive(Debug)]
pub struct EdgeGpuAdapter {
    pub(crate) ident: EdgeGpuAdapterId,
}

pub(crate) type EdgeGpuAdapterId = u64;

impl Drop for EdgeGpuAdapter {
    fn drop(&mut self) {
        unsafe {
            crate::webgpu_drop(self.ident);
        }
    }
}

impl wgpu::custom::AdapterInterface for EdgeGpuAdapter {
    fn request_device(&self, _desc: &wgpu::DeviceDescriptor<'_>) -> std::pin::Pin<Box<dyn wgpu::custom::RequestDeviceFuture>> {
        let mut device = 0u64;
        let mut queue = 0u64;

        assert!(unsafe { webgpu_adapter_device_create(self.ident, &mut device as *mut u64, &mut queue as *mut u64) > 0 });
        let device = crate::EdgeGpuDevice { ident: device };
        let queue = crate::EdgeGpuQueue { ident: queue };

        let res: Result<_, wgpu::RequestDeviceError> = Ok((wgpu::custom::DispatchDevice::custom(device), wgpu::custom::DispatchQueue::custom(queue)));
        Box::pin(std::future::ready(res))
    }

    fn is_surface_supported(&self, _surface: &wgpu::custom::DispatchSurface) -> bool {
        false
    }

    fn features(&self) -> wgpu::Features {
        wgpu::Features::all_webgpu_mask()
    }

    fn limits(&self) -> wgpu::Limits {
        wgpu::Limits::default()
    }

    fn downlevel_capabilities(&self) -> wgpu::DownlevelCapabilities {
        wgpu::DownlevelCapabilities::default()
    }

    fn get_info(&self) -> wgpu::AdapterInfo {
        log::info!("Called Adapter Info");
        wgpu::AdapterInfo {
            name: "Edgeless Virtual GPU".to_string(),
            vendor: 0,
            device: 0,
            device_type: wgpu::DeviceType::VirtualGpu,
            driver: "".to_string(),
            driver_info: "".to_string(),
            backend: wgpu::Backend::BrowserWebGpu,
        }
    }

    fn get_texture_format_features(&self, format: wgpu::TextureFormat) -> wgpu::TextureFormatFeatures {
        format.guaranteed_format_features(wgpu::custom::AdapterInterface::features(self))
    }

    fn get_presentation_timestamp(&self) -> wgpu::PresentationTimestamp {
        wgpu::PresentationTimestamp::INVALID_TIMESTAMP
    }
}
