// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

mod alloc_image;
mod static_image;

#[derive(Debug)]
pub enum CodeStoreError {
    NotFound,
    Exists,
    Incomplete,
    ReadOnly,
    InvalidWrite,
}

pub trait ImageRef {
    fn read(&self) -> &[u8];
}

pub trait ImageEntry {
    fn image<'a>(&'a self) -> Result<alloc::boxed::Box<dyn ImageRef + 'a>, CodeStoreError>;
    fn complete(&self) -> bool;
    fn update(&self, offset: usize, data: &[u8], complete: bool) -> Result<(), CodeStoreError>;
}

#[derive(Clone)]
pub struct ImageEntryContainer(alloc::sync::Arc<alloc::boxed::Box<dyn ImageEntry>>);

type StoreType = alloc::collections::BTreeMap<edgeless_api_core::function_instance::EncodedFunctionClassSpecification, ImageEntryContainer>;

#[derive(Clone)]
pub struct CodeStore {
    inner: &'static embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, StoreType>,
}

impl CodeStore {
    pub fn new() -> Self {
        Self {
            inner: alloc::boxed::Box::leak(alloc::boxed::Box::new(embassy_sync::mutex::Mutex::<
                embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                StoreType,
            >::new(StoreType::new()))),
        }
    }

    pub async fn has_image(&self, spec: &edgeless_api_core::function_instance::EncodedFunctionClassSpecification) -> bool {
        self.inner.lock().await.contains_key(spec)
    }

    pub async fn create_image(
        &mut self,
        spec: &edgeless_api_core::function_instance::EncodedFunctionClassSpecification,
    ) -> Result<ImageEntryContainer, CodeStoreError> {
        match self.inner.lock().await.entry(spec.clone()) {
            alloc::collections::btree_map::Entry::Vacant(vacant_entry) => {
                let entry = ImageEntryContainer(alloc::sync::Arc::new(alloc::boxed::Box::new(alloc_image::AllocImageEntry::new(
                    spec.image_size,
                ))));
                vacant_entry.insert(entry.clone());
                Ok(entry)
            }
            alloc::collections::btree_map::Entry::Occupied(_) => Err(CodeStoreError::Exists),
        }
    }

    pub async fn create_image_static(
        &mut self,
        spec: &edgeless_api_core::function_instance::EncodedFunctionClassSpecification,
        image: &'static [u8],
    ) -> Result<ImageEntryContainer, CodeStoreError> {
        match self.inner.lock().await.entry(spec.clone()) {
            alloc::collections::btree_map::Entry::Vacant(vacant_entry) => {
                let entry = ImageEntryContainer(alloc::sync::Arc::new(alloc::boxed::Box::new(static_image::StaticImageEntry { image })));
                vacant_entry.insert(entry.clone());
                Ok(entry)
            }
            alloc::collections::btree_map::Entry::Occupied(_) => Err(CodeStoreError::Exists),
        }
    }

    pub async fn get_image(
        &mut self,
        spec: &edgeless_api_core::function_instance::EncodedFunctionClassSpecification,
    ) -> Result<ImageEntryContainer, CodeStoreError> {
        match self.inner.lock().await.get(spec) {
            Some(v) => Ok(v.clone()),
            None => Err(CodeStoreError::NotFound),
        }
    }
}

impl ImageEntry for ImageEntryContainer {
    fn image<'a>(&'a self) -> Result<alloc::boxed::Box<dyn ImageRef + 'a>, CodeStoreError> {
        self.0.image()
    }

    fn complete(&self) -> bool {
        self.0.complete()
    }

    fn update(&self, offset: usize, data: &[u8], complete: bool) -> Result<(), CodeStoreError> {
        self.0.update(offset, data, complete)
    }
}
