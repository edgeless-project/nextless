#[async_trait::async_trait]
impl crate::proxy_instance::ProxyInstanceAPI for super::CoapClient {
    async fn start(&mut self, _request: crate::proxy_instance::ProxySpec) -> anyhow::Result<()> {
        Ok(())
    }
    async fn stop(&mut self, _id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        Ok(())
    }
    async fn patch(&mut self, _update: crate::proxy_instance::ProxySpec) -> anyhow::Result<()> {
        Ok(())
    }
}
