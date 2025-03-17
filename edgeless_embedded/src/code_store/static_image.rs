// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub(crate) struct StaticImageEntry {
    pub(crate) image: &'static [u8],
}

impl super::ImageEntry for StaticImageEntry {
    fn image<'a>(&'a self) -> Result<alloc::boxed::Box<dyn super::ImageRef + 'a>, super::CodeStoreError> {
        Ok(alloc::boxed::Box::new(self.image))
    }

    fn complete(&self) -> bool {
        true
    }

    fn update(&self, offset: usize, data: &[u8], complete: bool) -> Result<(), super::CodeStoreError> {
        Err(super::CodeStoreError::ReadOnly)
    }
}

impl super::ImageRef for &'static [u8] {
    fn read(&self) -> &[u8] {
        self
    }
}
