// SPDX-FileCopyrightText: © 2021 The gfx-rs developers
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
// See lib.rs for more information about the initial source.

#[derive(Debug)]
pub struct EdgeGpuMappedRange {
    pub(crate) buffer_id: u64,
    pub(crate) start: u64,
    pub(crate) end: u64,
    pub(crate) data: Vec<u8>,
    pub(crate) possibly_written: bool,
}

impl wgpu::custom::BufferMappedRangeInterface for EdgeGpuMappedRange {
    fn slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    fn slice_mut(&mut self) -> &mut [u8] {
        self.possibly_written = true;
        self.data.as_mut_slice()
    }
}

impl Drop for EdgeGpuMappedRange {
    fn drop(&mut self) {
        if self.possibly_written {
            unsafe {
                crate::buffer::webgpu_buffer_mapped_range_write(self.buffer_id, self.start, self.end, self.data.as_ptr());
            }
        }
    }
}
