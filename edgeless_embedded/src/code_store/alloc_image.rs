// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct AllocImageEntry {
    inner: core::cell::RefCell<alloc::vec::Vec<u8>>,
    complete: core::sync::atomic::AtomicBool,
}

struct AllocImageRef<'a> {
    lck: core::cell::Ref<'a, alloc::vec::Vec<u8>>,
}

impl AllocImageEntry {
    pub(crate) fn new(size: u64) -> Self {
        let mut buffer = alloc::vec::Vec::new();
        buffer.resize(size as usize, 0);

        Self {
            inner: core::cell::RefCell::new(buffer),
            complete: false.into(),
        }
    }
}

impl super::ImageEntry for AllocImageEntry {
    fn image<'a>(&'a self) -> Result<alloc::boxed::Box<dyn super::ImageRef + 'a>, super::CodeStoreError> {
        if !self.complete.load(core::sync::atomic::Ordering::Relaxed) {
            return Err(super::CodeStoreError::Incomplete);
        }

        let image_ref = alloc::boxed::Box::new(AllocImageRef { lck: self.inner.borrow() });

        Ok(image_ref)
    }

    fn complete(&self) -> bool {
        self.complete.load(core::sync::atomic::Ordering::Relaxed)
    }

    fn update(&self, offset: usize, data: &[u8], complete: bool) -> Result<(), super::CodeStoreError> {
        if self.complete.load(core::sync::atomic::Ordering::Relaxed) {
            return Err(super::CodeStoreError::ReadOnly);
        }

        match self.inner.try_borrow_mut() {
            Ok(mut inner) => {
                inner[offset..offset + data.len()].copy_from_slice(data);
                if complete {
                    self.complete.store(true, core::sync::atomic::Ordering::Relaxed);
                }
                Ok(())
            }
            Err(_) => Err(super::CodeStoreError::InvalidWrite),
        }
    }
}

impl super::ImageRef for AllocImageRef<'_> {
    fn read(&self) -> &[u8] {
        self.lck.as_slice()
    }
}
