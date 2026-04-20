// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

mod host_api;
mod loader;

static ARENA_LEN: usize = 128 * 1024 * 1024;

pub struct NativeFunctionInstance {
    fns: edgeless_actor_abi::GuestApi<'static>,
    host_api: *mut host_api::HostApiImpl,
    // Stored to ensure the memory is cleaned up.
    _actor: loader::LoadedActor,
    buffer: *mut u8,
}

impl Drop for NativeFunctionInstance {
    fn drop(&mut self) {
        core::mem::drop(unsafe { Box::from_raw(self.host_api) });
        unsafe {
            rustix::mm::munmap(self.buffer as *mut std::ffi::c_void, ARENA_LEN).unwrap();
        }
    }
}

unsafe impl Send for NativeFunctionInstance {}

impl crate::base_runtime::FunctionInstanceSync for NativeFunctionInstance {
    fn instantiate(
        _instance_id: &edgeless_api::function_instance::InstanceId,
        _runtime_configuration: std::collections::HashMap<String, String>,
        guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
        code: &[u8],
    ) -> crate::base_runtime::FunctionInstanceResult<Box<Self>> {
        // To do this securely, the compiler needs to enforce certain rules.
        // The system must also ensure the binary comes from a compiler enforcing those rules.
        // https://www.usenix.org/conference/atc18/presentation/boucher
        // https://dl.acm.org/doi/10.1145/3477113.3487272
        tracing::warn!("Security Warning: Starting a native actor. Nextless is currently missing security features and this is insecure.");
        unsafe {
            let buffer = rustix::mm::mmap_anonymous(
                std::ptr::null_mut(),
                ARENA_LEN,
                rustix::mm::ProtFlags::WRITE | rustix::mm::ProtFlags::READ,
                rustix::mm::MapFlags::PRIVATE,
            )
            .unwrap() as *mut u8;
            let mut alloc = talc::Talc::new(talc::ErrOnOom);
            alloc
                .claim(talc::Span::from_base_size(buffer, ARENA_LEN))
                .map_err(|_| crate::base_runtime::FunctionInstanceError::Internal(anyhow::anyhow!("Could not claim Memory")))?;

            // The elfloader / xmas-elf require the parsed elf to reside on aligned memory.
            // This temporarily places the raw elf into aligned memory.
            let mut actor = {
                let aligned_code = rustix::mm::mmap_anonymous(
                    std::ptr::null_mut(),
                    code.len(),
                    rustix::mm::ProtFlags::WRITE | rustix::mm::ProtFlags::READ,
                    rustix::mm::MapFlags::PRIVATE,
                )
                .unwrap();
                let aligned_code_slice = std::ptr::slice_from_raw_parts_mut::<u8>(aligned_code.cast(), code.len());
                aligned_code_slice.as_mut().unwrap().clone_from_slice(code);

                let binary = elfloader::ElfBinary::new(aligned_code_slice.as_mut().unwrap()).unwrap();

                let mut loader = loader::ActorLoader::new();

                binary.load(&mut loader).unwrap();

                let actor = loader.finalize(
                    binary
                        .get_sym_offset("init")
                        .map_err(|_| crate::base_runtime::FunctionInstanceError::BadCode(anyhow::anyhow!("No Init Symbol")))?,
                );

                rustix::mm::munmap(aligned_code, code.len()).unwrap();
                actor
            };

            let api = Box::into_raw(Box::new(host_api::HostApiImpl {
                host: guest_api_host,
                alloc: alloc.lock::<spin::Mutex<()>>(),
            }));

            let handler = actor.instantiate(api.as_mut().unwrap());

            Ok(Box::new(Self {
                buffer,
                _actor: actor,
                host_api: api,
                fns: handler,
            }))
        }
    }

    fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&[u8]>) -> crate::base_runtime::FunctionInstanceResult<()> {
        (self.fns.handle_init)(init_payload.map(|s| s.as_bytes()), serialized_state)
            .map_err(|e| crate::base_runtime::FunctionInstanceError::BadCode(e.into()))
    }

    fn cast(&mut self, src: &edgeless_api::function_instance::InstanceId, port: &str, msg: &[u8]) -> crate::base_runtime::FunctionInstanceResult<()> {
        (self.fns.handle_cast)(
            edgeless_actor_abi::ActorId {
                node_id: src.node_id.into_bytes(),
                component_id: src.function_id.into_bytes(),
            },
            port,
            msg,
        )
        .map_err(|e| crate::base_runtime::FunctionInstanceError::BadCode(e.into()))
    }

    fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        port: &str,
        msg: &[u8],
    ) -> crate::base_runtime::FunctionInstanceResult<crate::dataplane::core::CallRet> {
        (self.fns.handle_call)(
            edgeless_actor_abi::ActorId {
                node_id: src.node_id.into_bytes(),
                component_id: src.function_id.into_bytes(),
            },
            port,
            msg,
        )
        .map(|ret| match ret {
            edgeless_actor_abi::CallRet::NoReply => crate::dataplane::core::CallRet::NoReply,
            edgeless_actor_abi::CallRet::Reply(data) => crate::dataplane::core::CallRet::Reply(Vec::from(data.as_slice())),
            edgeless_actor_abi::CallRet::Err => crate::dataplane::core::CallRet::Err,
        })
        .map_err(|e| crate::base_runtime::FunctionInstanceError::BadCode(e.into()))
    }

    fn stop(&mut self) -> crate::base_runtime::FunctionInstanceResult<()> {
        (self.fns.handle_stop)().map_err(|e| crate::base_runtime::FunctionInstanceError::BadCode(e.into()))
    }
}
