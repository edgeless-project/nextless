// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
// See lib.rs for more information about the initial source.

extern "C" {
    fn webgpu_buffer_map_async(buffer_id: u64, mode: u64, start: u64, end: u64);
    fn webgpu_buffer_unmap(buffer_id: u64);
    fn webgpu_buffer_mapped_range_read(buffer_id: u64, sub_range_start: u64, sub_range_end: u64, data: *mut u8);
    pub(crate) fn webgpu_buffer_mapped_range_write(buffer_id: u64, sub_range_start: u64, sub_range_end: u64, data: *const u8);
}

#[derive(Debug)]
pub struct EdgeGpuBuffer {
    pub(crate) ident: u64,
}

impl wgpu::custom::BufferInterface for EdgeGpuBuffer {
    fn map_async(&self, mode: wgpu::MapMode, range: std::ops::Range<wgpu::BufferAddress>, _callback: wgpu::custom::BufferMapCallback) {
        let mode: u64 = match mode {
            wgpu::MapMode::Read => 1,
            wgpu::MapMode::Write => 2,
        };

        let start = range.start;
        let end = range.end;

        log::info!("Ignored Map Async Callback");

        unsafe { webgpu_buffer_map_async(self.ident, mode, start, end) }
    }

    fn get_mapped_range(&self, sub_range: std::ops::Range<wgpu::BufferAddress>) -> wgpu::custom::DispatchBufferMappedRange {
        let data_len = sub_range.end - sub_range.start;
        let mut data = vec![0u8; data_len as usize];

        unsafe {
            webgpu_buffer_mapped_range_read(self.ident, sub_range.start, sub_range.end, data.as_mut_ptr());
        }

        wgpu::custom::DispatchBufferMappedRange::custom(crate::EdgeGpuMappedRange {
            buffer_id: self.ident,
            start: sub_range.start,
            end: sub_range.end,
            data,
            possibly_written: false,
        })
    }

    fn unmap(&self) {
        unsafe {
            webgpu_buffer_unmap(self.ident);
        }
    }

    fn destroy(&self) {
        log::info!("Called Destroy");
    }
}
