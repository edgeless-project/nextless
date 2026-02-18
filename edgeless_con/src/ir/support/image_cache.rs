// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Default)]
pub struct ImageCache {
    inner: std::sync::Arc<tokio::sync::Mutex<ImageCacheInner>>,
}

#[derive(Default)]
struct ImageCacheInner {
    images: std::collections::HashMap<crate::ir::behavior::BehaviorId, Vec<crate::ir::behavior::BehaviorImage>>,
}

pub enum CacheResult {
    NotFound,
    PartialMatch(PartialMatch),
    FullMatch(crate::ir::behavior::BehaviorImage),
}

#[derive(Clone)]
pub struct PartialMatch {
    images: Vec<crate::ir::behavior::BehaviorImage>,
}

impl From<PartialMatch> for Vec<crate::ir::behavior::BehaviorImage> {
    fn from(val: PartialMatch) -> Self {
        val.images
    }
}

impl PartialMatch {
    pub fn same_runtime_feature_subset(self, other: &crate::ir::behavior::BehaviorImageId) -> PartialMatch {
        PartialMatch {
            images: self
                .images
                .into_iter()
                .filter(|i| {
                    if i.behavior_image_id.dialect_type.base_type != other.dialect_type.base_type {
                        return false;
                    }

                    i.behavior_image_id.dialect_type.features.is_subset(&other.dialect_type.features)
                })
                .collect(),
        }
    }

    pub fn same_or_more_ports(self, other: &crate::ir::behavior::BehaviorImageId) -> PartialMatch {
        PartialMatch {
            images: self
                .images
                .into_iter()
                .filter(|i| {
                    i.behavior_image_id
                        .enabled_ports
                        .enabled_inputs
                        .is_superset(&other.enabled_ports.enabled_inputs)
                        && i.behavior_image_id
                            .enabled_ports
                            .enabled_outputs
                            .is_superset(&other.enabled_ports.enabled_outputs)
                })
                .collect(),
        }
    }
}

impl ImageCache {
    // This is actually used by tests.
    #[allow(unused)]
    pub fn new() -> Self {
        Self { inner: Default::default() }
    }

    pub async fn get(&self, ident: &crate::ir::behavior::BehaviorImageId) -> CacheResult {
        let lck = self.inner.lock().await;

        // https://stackoverflow.com/a/75486197
        let Some(images) = lck.images.get(&ident.behavior_id) else {
            return CacheResult::NotFound;
        };

        if let Some(image) = images.iter().find(|image| &image.behavior_image_id == ident) {
            return CacheResult::FullMatch(image.clone());
        }

        CacheResult::PartialMatch(PartialMatch { images: images.to_vec() })
    }

    pub async fn insert(&self, image: crate::ir::behavior::BehaviorImage) {
        let mut lck = self.inner.lock().await;

        let images = lck.images.entry(image.behavior_image_id.behavior_id.clone()).or_insert(Vec::new());
        images.retain(|existing| existing.behavior_image_id != image.behavior_image_id);
        images.push(image);
    }

    pub fn get_blocking(&self, ident: &crate::ir::behavior::BehaviorImageId) -> CacheResult {
        tokio::runtime::Handle::current().block_on(self.get(ident))
    }

    pub fn insert_blocking(&self, image: crate::ir::behavior::BehaviorImage) {
        tokio::runtime::Handle::current().block_on(self.insert(image))
    }
}
