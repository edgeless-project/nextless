// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[async_trait::async_trait]
impl crate::link::LinkInstanceAPI for crate::coap_impl::CoapClient {
    async fn create(&mut self, _req: crate::link::CreateLinkRequest) -> anyhow::Result<()> {
        Ok(())
    }
    async fn remove(&mut self, _id: crate::link::LinkInstanceId) -> anyhow::Result<()> {
        Ok(())
    }
}
