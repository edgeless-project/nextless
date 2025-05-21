// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

#[unsafe(no_mangle)]
pub static mut HOST_API: Option<&'static mut dyn edgeless_actor_abi::HostApi> = None;

// https://github.com/rust-lang/rust/issues/44871#issuecomment-2404152302
#[cfg(not(feature = "std"))]
#[link(name = "System", kind = "dylib")]
extern "C" {}

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
