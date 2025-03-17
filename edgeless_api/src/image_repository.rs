// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub trait FunctionImageHash {
    fn image_hash(&self) -> [u8; 32];
}

impl FunctionImageHash for std::vec::Vec<u8> {
    fn image_hash(&self) -> [u8; 32] {
        let mut context = ring::digest::Context::new(&ring::digest::SHA256);
        context.update(&self[..]);
        context.finish().as_ref().try_into().unwrap()
    }
}

#[async_trait::async_trait]
pub trait ImageRepositoryAPI: ImageRepositoryAPIClone + Send + Sync {
    async fn get(&self, hash: [u8; 32]) -> Option<Vec<u8>>;
    async fn get_part(&self, hash: [u8; 32], offset: usize, size: usize) -> Option<(Vec<u8>, bool)>;
}

// https://stackoverflow.com/a/30353928
pub trait ImageRepositoryAPIClone {
    fn clone_box(&self) -> Box<dyn ImageRepositoryAPI>;
}
impl<T> ImageRepositoryAPIClone for T
where
    T: 'static + ImageRepositoryAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn ImageRepositoryAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn ImageRepositoryAPI> {
    fn clone(&self) -> Box<dyn ImageRepositoryAPI> {
        self.clone_box()
    }
}
