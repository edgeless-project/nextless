// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-FileCopyrightText: © 2016 Gerd Zellweger
// SPDX-License-Identifier: MIT

// Based on the example loader in https://github.com/gz/rust-elfloader
// Usage of rustix for mmap/mprotect inspired by wasmtime (darwin does not allow concurrent Write&Execute).

const PAGE_SIZE: usize = 0x10000;

pub(crate) struct ActorLoader {
    data: *mut u8,
    len: usize,
    sections: Vec<Section>,
}

pub(crate) struct LoadedActor {
    data: *mut u8,
    len: usize,
    init_fn: *mut u64,
}

struct Section {
    virt_addr: usize,
    len: usize,
    protection: rustix::mm::MprotectFlags,
}

impl elfloader::ElfLoader for ActorLoader {
    fn allocate(&mut self, load_headers: elfloader::LoadableHeaders) -> Result<(), elfloader::ElfLoaderErr> {
        let mut max = 0;

        for header in load_headers {
            max = std::cmp::max(max, header.virtual_addr() + header.mem_size());

            let mut prot_flags = rustix::mm::MprotectFlags::READ;

            if header.flags().is_execute() {
                prot_flags |= rustix::mm::MprotectFlags::EXEC;
            } else if header.flags().is_write() {
                prot_flags |= rustix::mm::MprotectFlags::WRITE;
            }
            self.sections.push(Section {
                virt_addr: header.virtual_addr() as usize,
                len: header.mem_size() as usize,
                protection: prot_flags,
            });
        }

        unsafe {
            // TODO(raphaelhetzel): Check use of rustix::mm::MapFlags::NORESERVE (used by wasmtime)
            let segment = rustix::mm::mmap_anonymous(
                std::ptr::null_mut(),
                max as usize,
                rustix::mm::ProtFlags::WRITE | rustix::mm::ProtFlags::READ,
                rustix::mm::MapFlags::PRIVATE,
            )
            .unwrap();
            self.data = segment.cast();
            self.len = max as usize;
        }

        Ok(())
    }

    fn relocate(&mut self, entry: elfloader::RelocationEntry) -> Result<(), elfloader::ElfLoaderErr> {
        unsafe {
            let dst: *mut *mut u8 = self.data.add((entry.offset) as usize).cast();
            match entry.rtype {
                elfloader::RelocationType::AArch64(elfloader::arch::aarch64::RelocationTypes::R_AARCH64_RELATIVE) => {
                    let addend = entry.addend.ok_or(elfloader::ElfLoaderErr::UnsupportedRelocationEntry)?;
                    let t: *mut u8 = self.data.add(addend as usize).cast();
                    *dst = t;
                    Ok(())
                }
                unknown_rel => {
                    log::info!("Found unknown Relocation Entry: {unknown_rel:?}");
                    Ok(())
                }
            }
        }
    }

    fn load(&mut self, _flags: elfloader::Flags, base: elfloader::VAddr, region: &[u8]) -> Result<(), elfloader::ElfLoaderErr> {
        unsafe {
            let slice = std::ptr::slice_from_raw_parts_mut::<u8>(self.data.add(base as usize), region.len())
                .as_mut()
                .unwrap();

            slice.copy_from_slice(region);
        }
        Ok(())
    }

    fn tls(&mut self, tdata_start: elfloader::VAddr, tdata_length: u64, total_size: u64, align: u64) -> Result<(), elfloader::ElfLoaderErr> {
        log::error!("Unexpeted/Unimplemented use of Loader TLS. start: {tdata_start}, len: {tdata_length}, total: {total_size}, align: {align}");
        Err(elfloader::ElfLoaderErr::UnsupportedAbi)
    }
}

impl ActorLoader {
    pub(crate) fn new() -> Self {
        Self {
            data: core::ptr::null_mut(),
            len: 0,
            sections: Vec::new(),
        }
    }

    pub(crate) fn finalize(self, init_offset: u64) -> LoadedActor {
        unsafe {
            for s in &self.sections {
                rustix::mm::mprotect(
                    self.data.add(s.virt_addr / PAGE_SIZE * PAGE_SIZE).cast(),
                    (s.virt_addr - (s.virt_addr / PAGE_SIZE * PAGE_SIZE)) + s.len,
                    s.protection,
                )
                .unwrap();
            }

            LoadedActor {
                data: self.data,
                len: self.len,
                init_fn: self.data.add((init_offset) as usize).cast(),
            }
        }
    }
}

impl LoadedActor {
    pub(crate) fn instantiate<'a, 'b>(&'a mut self, host_api: &'b mut impl edgeless_actor_abi::HostApi<'b>) -> edgeless_actor_abi::GuestApi<'b> {
        // Transmute to get a fn(): https://kerkour.com/rust-execute-from-memory
        let f: edgeless_actor_abi::ActorInit = unsafe { std::mem::transmute(self.init_fn) };
        (f)(host_api)
    }
}

impl Drop for LoadedActor {
    fn drop(&mut self) {
        unsafe { rustix::mm::munmap(self.data.cast(), self.len).unwrap() };
    }
}
