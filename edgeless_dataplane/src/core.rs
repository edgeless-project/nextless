// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
pub use edgeless_api::invocation::LinkProcessingResult;

/// Trait that needs to be implemented by each link that is added to a dataplane chain.
/// Link instances are commonly created by a LinkProvider (which is not a trait yet).
#[async_trait::async_trait]
#[allow(clippy::too_many_arguments)]
pub trait DataPlaneLink: Send + Sync {
    async fn handle_send(
        &mut self,
        target: &edgeless_api::function_instance::InstanceId,
        msg: Message,
        src: &edgeless_api::function_instance::InstanceId,
        channel_id: u64,
        target_port: edgeless_api::function_instance::PortId,
        source_port: edgeless_api::function_instance::PortId,
        context: opentelemetry::trace::SpanContext,
    ) -> LinkProcessingResult;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallRet {
    NoReply,
    Reply(Vec<u8>),
    Err,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Message {
    Cast(Vec<u8>),
    Call(Vec<u8>),
    CallRet(Vec<u8>),
    CallNoRet,
    Err,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Cast(data) => f.write_fmt(format_args!(
                "Cast(HEAD: {:?}, LEN: {})",
                &data[..std::cmp::min(data.len(), 20)],
                data.len()
            )),
            Message::Call(data) => f.write_fmt(format_args!(
                "Call(HEAD: {:?}, LEN: {})",
                &data[..std::cmp::min(data.len(), 20)],
                data.len()
            )),
            Message::CallRet(data) => f.write_fmt(format_args!(
                "CallReply(HEAD: {:?}, LEN: {})",
                &data[..std::cmp::min(data.len(), 20)],
                data.len()
            )),
            Message::CallNoRet => f.write_str("CallReplyEmpty"),
            Message::Err => f.write_str("CallReplyErr"),
        }
    }
}

impl Message {
    pub fn payload_len(&self) -> usize {
        match self {
            Message::Cast(data) => data.len(),
            Message::Call(data) => data.len(),
            Message::CallRet(data) => data.len(),
            Message::CallNoRet => 0,
            Message::Err => 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DataplaneEvent {
    pub source_id: edgeless_api::function_instance::InstanceId,
    pub source_port: edgeless_api::function_instance::PortId,
    pub channel_id: u64,
    pub message: Message,
    pub target_port: edgeless_api::function_instance::PortId,
    pub context: opentelemetry::trace::SpanContext,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessDataplanePeerSettings {
    pub node_id: uuid::Uuid,
    pub invocation_url: String,
}
