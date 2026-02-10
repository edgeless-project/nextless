// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use crate::invocation::InvocationAPI;

#[derive(Clone, Debug)]
pub enum DataplaneError {
    BadParameter,
    Internal,
    UnknownAlias,
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallRet {
    NoReply,
    Reply(heapless::Vec<u8, 1500>),
    Err,
}

pub struct EmbeddedDataplaneHandle {
    own_id: edgeless_api_core::instance_id::InstanceId,
    inner: embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, EmbeddedDataplaneHandleInner>,
}

struct EmbeddedDataplaneHandleInner {
    pub agent: crate::agent::EmbeddedAgent,
    alias_mapping: AliasMapping,
}

struct AliasMapping {
    outputs: heapless::Vec<(edgeless_api_core::port::Port<32>, edgeless_api_core::common::Output), 16>,
}

impl EmbeddedDataplaneHandle {
    pub fn new(
        own_id: edgeless_api_core::instance_id::InstanceId,
        agent: crate::agent::EmbeddedAgent,
        output_mapping: heapless::Vec<(edgeless_api_core::port::Port<32>, edgeless_api_core::common::Output), 16>,
    ) -> Self {
        Self {
            own_id,
            inner: embassy_sync::mutex::Mutex::new(EmbeddedDataplaneHandleInner {
                agent,
                alias_mapping: AliasMapping { outputs: output_mapping },
            }),
        }
    }

    pub async fn patch(&mut self, output_mapping: heapless::Vec<(edgeless_api_core::port::Port<32>, edgeless_api_core::common::Output), 16>) {
        self.inner.lock().await.alias_mapping.outputs = output_mapping;
    }

    pub async fn send_alias(&self, alias: &str, msg: &[u8]) -> Result<(), DataplaneError> {
        let outputs = self
            .inner
            .lock()
            .await
            .alias_mapping
            .outputs
            .iter()
            .find(|port_id| port_id.0 .0 == alias)
            .map(|o| o.1.clone())
            .ok_or(DataplaneError::UnknownAlias)?;

        match outputs {
            edgeless_api_core::common::Output::Single(id) => {
                self.send(self.own_id, id.instance_id, id.port_id.clone(), msg).await?;
            }
            edgeless_api_core::common::Output::Any(ids) => {
                let id = ids.0.first();
                if let Some(id) = id {
                    self.send(self.own_id, id.instance_id, id.port_id.clone(), msg).await?;
                } else {
                    return Err(DataplaneError::UnknownAlias);
                }
            }
            edgeless_api_core::common::Output::All(ids) => {
                if ids.0.is_empty() {
                    return Err(DataplaneError::UnknownAlias);
                }
                for id in ids.0 {
                    self.send(self.own_id, id.instance_id, id.port_id.clone(), msg).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn send(
        &self,
        slf: edgeless_api_core::instance_id::InstanceId,
        target: edgeless_api_core::instance_id::InstanceId,
        target_port: edgeless_api_core::port::Port<32>,
        msg: &[u8],
    ) -> Result<(), DataplaneError> {
        let event = edgeless_api_core::invocation::Event {
            target,
            source: slf,
            target_port,
            source_port: edgeless_api_core::port::Port("UNKNOWN".try_into().map_err(|_| DataplaneError::Internal)?),
            stream_id: 0,
            data: edgeless_api_core::invocation::EventData::Cast(edgeless_api_core::invocation::DataBuffer(
                heapless::Vec::<u8, 1500>::from_slice(msg).map_err(|_| DataplaneError::BadParameter)?,
            )),
            span_context: edgeless_api_core::invocation::SpanContext {
                trace_id: [0; 16],
                span_id: [0; 8],
                trace_flags: 0,
            },
        };
        self.inner.lock().await.agent.handle(event).await.map_err(|_| DataplaneError::Internal)?;
        Ok(())
    }

    pub async fn reply(
        &self,
        slf: edgeless_api_core::instance_id::InstanceId,
        target: edgeless_api_core::instance_id::InstanceId,
        target_channel: u64,
        msg: CallRet,
    ) -> Result<(), DataplaneError> {
        let event = edgeless_api_core::invocation::Event {
            target,
            source: slf,
            target_port: edgeless_api_core::port::Port("UNKNOWN".try_into().map_err(|_| DataplaneError::Internal)?),
            source_port: edgeless_api_core::port::Port("UNKNOWN".try_into().map_err(|_| DataplaneError::Internal)?),
            stream_id: target_channel,
            data: match msg {
                CallRet::Reply(msg) => edgeless_api_core::invocation::EventData::CallRet(edgeless_api_core::invocation::DataBuffer(msg)),
                CallRet::NoReply => edgeless_api_core::invocation::EventData::CallNoRet,
                CallRet::Err => edgeless_api_core::invocation::EventData::Err,
            },
            span_context: edgeless_api_core::invocation::SpanContext {
                trace_id: [0; 16],
                span_id: [0; 8],
                trace_flags: 0,
            },
        };
        self.inner.lock().await.agent.handle(event).await.map_err(|_| DataplaneError::Internal)?;
        Ok(())
    }
}
