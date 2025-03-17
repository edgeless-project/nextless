// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub struct ImageRepository {
    store: std::sync::Arc<tokio::sync::RwLock<Inner>>,
}

type Inner = std::collections::HashMap<[u8; 32], Vec<u8>>;

impl ImageRepository {
    pub fn new() -> Self {
        Self {
            store: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn update(&self, hash: [u8; 32], image: Vec<u8>) {
        self.store.write().await.insert(hash, image);
    }
}

#[async_trait::async_trait]
impl edgeless_api::image_repository::ImageRepositoryAPI for ImageRepository {
    async fn get(&self, hash: [u8; 32]) -> Option<Vec<u8>> {
        Some(self.store.read().await.get(&hash)?.clone())
    }

    async fn get_part(&self, hash: [u8; 32], offset: usize, size: usize) -> Option<(Vec<u8>, bool)> {
        let lck = self.store.read().await;

        let item = lck.get(&hash)?;

        if offset > item.len() {
            return None;
        }

        if offset + size >= item.len() {
            Some((item[offset..].to_vec(), true))
        } else {
            Some((item[offset..offset + size].to_vec(), false))
        }
    }
}
