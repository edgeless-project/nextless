// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

/// # Safety
///
/// This can only be called on valid pointers. Only used in WASM.
pub unsafe extern "C" fn edgeless_mem_alloc(payload_len: usize) -> *mut u8 {
    let align = core::mem::align_of::<usize>();
    let layout = core::alloc::Layout::from_size_align_unchecked(payload_len, align);
    allocator_api2::alloc::alloc(layout)
}

/// # Safety
///
/// This can only be called on valid pointers. Only used in WASM.
pub unsafe extern "C" fn edgeless_mem_clear() {
    // We always free and clear, so this does not leak memory.
}

/// # Safety
///
/// This can only be called on valid pointers. Only used in WASM.
pub unsafe extern "C" fn edgeless_mem_free(ptr: *mut u8, size: usize) {
    let align = core::mem::align_of::<usize>();
    let layout = core::alloc::Layout::from_size_align_unchecked(size, align);
    // ALLOCATOR.dealloc(ptr, layout);
    allocator_api2::alloc::dealloc(ptr, layout);
}

pub struct EdgelessGlobalAlloc {}

unsafe impl Sync for EdgelessGlobalAlloc {}

unsafe impl core::alloc::GlobalAlloc for EdgelessGlobalAlloc {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if let Some(host_api) = crate::interface_dynlib::HOST_API.as_mut() {
            if let Ok(allocation) = host_api.allocator().allocate(layout) {
                return allocation.as_ptr().cast();
            } else {
                crate::output_api::telemetry_log(1, "ALLOCATOR", "alloc failed");
            }
        } else {
            crate::output_api::telemetry_log(1, "ALLOCATOR", "no host_api");
        }
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        crate::interface_dynlib::HOST_API
            .as_mut()
            .unwrap()
            .allocator()
            .deallocate(core::ptr::NonNull::new_unchecked(ptr), layout);
    }
}

#[cfg(not(feature = "std"))]
#[global_allocator]
static ALLOCATOR: EdgelessGlobalAlloc = EdgelessGlobalAlloc {};
