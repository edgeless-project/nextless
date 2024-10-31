// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub struct LinkInstanceAPIClient {
    client: crate::grpc_impl::api::link_instance_client::LinkInstanceClient<tonic::transport::Channel>,
}

#[async_trait::async_trait]
impl crate::link::LinkInstanceAPI for LinkInstanceAPIClient {
    async fn create(&mut self, req: crate::link::CreateLinkRequest) -> anyhow::Result<()> {
        match self.client.create(tonic::Request::new(req.into())).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Request Failed")),
        }
    }
    async fn remove(&mut self, id: crate::link::LinkInstanceId) -> anyhow::Result<()> {
        match self.client.remove(tonic::Request::new(id.into())).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Request Failed")),
        }
    }
}

impl LinkInstanceAPIClient {
    pub async fn new(server_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        loop {
            match crate::grpc_impl::api::link_instance_client::LinkInstanceClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Ok(Self { client });
                }
                Err(err) => match retry_interval {
                    Some(val) => tokio::time::sleep(tokio::time::Duration::from_secs(val)).await,
                    None => {
                        return Err(anyhow::anyhow!("Error when connecting to {}: {}", server_addr, err));
                    }
                },
            }
        }
    }
}

pub struct LinkInstanceServerHandler {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::link::LinkInstanceAPI>>,
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::link_instance_server::LinkInstance for LinkInstanceServerHandler {
    async fn create(&self, req: tonic::Request<crate::grpc_impl::api::CreateLinkInstanceRequest>) -> tonic::Result<tonic::Response<()>> {
        let inner = req.into_inner();

        let parsed = crate::link::CreateLinkRequest::try_from(inner).unwrap();

        self.root_api
            .lock()
            .await
            .create(parsed)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        Ok(tonic::Response::new(()))
    }

    async fn remove(&self, req: tonic::Request<crate::grpc_impl::api::LinkInstanceId>) -> tonic::Result<tonic::Response<()>> {
        let inner = req.into_inner();

        let parsed = crate::link::LinkInstanceId::try_from(inner).unwrap();

        self.root_api
            .lock()
            .await
            .remove(parsed)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        Ok(tonic::Response::new(()))
    }
}

impl TryFrom<crate::grpc_impl::api::CreateLinkInstanceRequest> for crate::link::CreateLinkRequest {
    type Error = anyhow::Error;

    fn try_from(value: crate::grpc_impl::api::CreateLinkInstanceRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            id: crate::grpc_impl::common::CommonConverters::parse_link_id(&value.id.ok_or(anyhow::anyhow!("Missing Field"))?)?,
            provider: crate::link::LinkProviderId::try_from(value.provider_id.ok_or(anyhow::anyhow!("Missing Field"))?)?,
            config: value.config,
            direction: crate::link::LinkDirection::try_from(
                crate::grpc_impl::api::LinkDirection::from_i32(value.direction).ok_or(anyhow::anyhow!("Bad Enum"))?,
            )?,
        })
    }
}

impl From<crate::link::CreateLinkRequest> for crate::grpc_impl::api::CreateLinkInstanceRequest {
    fn from(val: crate::link::CreateLinkRequest) -> Self {
        let direction: crate::grpc_impl::api::LinkDirection = val.direction.into();

        crate::grpc_impl::api::CreateLinkInstanceRequest {
            id: Some(val.id.into()),
            provider_id: Some(val.provider.into()),
            config: val.config,
            direction: direction as i32,
        }
    }
}

impl TryFrom<crate::grpc_impl::api::LinkProviderId> for crate::link::LinkProviderId {
    type Error = anyhow::Error;

    fn try_from(value: crate::grpc_impl::api::LinkProviderId) -> Result<Self, Self::Error> {
        Ok(Self(uuid::Uuid::parse_str(&value.id)?))
    }
}

impl From<crate::link::LinkInstanceId> for crate::grpc_impl::api::LinkInstanceId {
    fn from(val: crate::link::LinkInstanceId) -> Self {
        crate::grpc_impl::api::LinkInstanceId { id: val.0.to_string() }
    }
}

impl TryFrom<crate::grpc_impl::api::LinkDirection> for crate::link::LinkDirection {
    type Error = anyhow::Error;

    fn try_from(value: crate::grpc_impl::api::LinkDirection) -> Result<Self, Self::Error> {
        Ok(match value {
            crate::grpc_impl::api::LinkDirection::Read => Self::Read,
            crate::grpc_impl::api::LinkDirection::Write => Self::Write,
            crate::grpc_impl::api::LinkDirection::BiDi => Self::BiDi,
        })
    }
}

impl From<crate::link::LinkDirection> for crate::grpc_impl::api::LinkDirection {
    fn from(val: crate::link::LinkDirection) -> Self {
        match val {
            crate::link::LinkDirection::Read => super::api::LinkDirection::Read,
            crate::link::LinkDirection::Write => super::api::LinkDirection::Write,
            crate::link::LinkDirection::BiDi => super::api::LinkDirection::BiDi,
        }
    }
}

impl TryFrom<crate::grpc_impl::api::LinkInstanceId> for crate::link::LinkInstanceId {
    type Error = anyhow::Error;

    fn try_from(value: crate::grpc_impl::api::LinkInstanceId) -> Result<Self, Self::Error> {
        Ok(Self(uuid::Uuid::parse_str(&value.id)?))
    }
}

impl From<crate::link::LinkProviderId> for crate::grpc_impl::api::LinkProviderId {
    fn from(val: crate::link::LinkProviderId) -> Self {
        crate::grpc_impl::api::LinkProviderId { id: val.0.to_string() }
    }
}

impl TryFrom<crate::grpc_impl::api::LinkType> for crate::link::LinkType {
    type Error = anyhow::Error;

    fn try_from(value: crate::grpc_impl::api::LinkType) -> Result<Self, Self::Error> {
        Ok(Self(value.r#type))
    }
}

impl From<crate::link::LinkType> for crate::grpc_impl::api::LinkType {
    fn from(val: crate::link::LinkType) -> Self {
        crate::grpc_impl::api::LinkType { r#type: val.0.to_string() }
    }
}
