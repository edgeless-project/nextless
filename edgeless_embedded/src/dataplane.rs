// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use crate::invocation::InvocationAPI;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallRet {
    NoReply,
    Reply(alloc::vec::Vec<u8>),
    Err,
}

pub struct EmbeddedDataplaneHandle {
    own_id: edgeless_api_core::instance_id::InstanceId,
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
            agent,
            alias_mapping: AliasMapping { outputs: output_mapping },
        }
    }

    pub fn patch(&mut self, output_mapping: heapless::Vec<(edgeless_api_core::port::Port<32>, edgeless_api_core::common::Output), 16>) {
        self.alias_mapping.outputs = output_mapping;
    }

    pub async fn send_alias(&mut self, alias: &str, msg: &[u8]) {
        let outputs = self
            .alias_mapping
            .outputs
            .iter()
            .find(|port_id| port_id.0 .0 == alias)
            .map(|o| o.1.clone())
            .unwrap();

        match outputs {
            edgeless_api_core::common::Output::Single(id) => {
                self.send(self.own_id, id.instance_id, id.port_id.clone(), &msg).await;
            }
            edgeless_api_core::common::Output::Any(ids) => {
                let id = ids.0.first();
                if let Some(id) = id {
                    self.send(self.own_id, id.instance_id, id.port_id.clone(), &msg).await;
                } else {
                    // return Err(GuestAPIError::UnknownAlias)
                }
            }
            edgeless_api_core::common::Output::All(ids) => {
                for id in ids.0 {
                    self.send(self.own_id, id.instance_id, id.port_id.clone(), &msg).await;
                }
            }
        }
    }

    pub async fn send(
        &mut self,
        slf: edgeless_api_core::instance_id::InstanceId,
        target: edgeless_api_core::instance_id::InstanceId,
        target_port: edgeless_api_core::port::Port<32>,
        msg: &[u8],
    ) {
        let event = edgeless_api_core::invocation::Event {
            target,
            source: slf,
            target_port,
            stream_id: 0,
            data: edgeless_api_core::invocation::EventData::Cast(edgeless_api_core::invocation::DataBuffer(
                heapless::Vec::<u8, 1500>::from_slice(msg).unwrap(),
            )),
            span_context: edgeless_api_core::invocation::SpanContext {
                trace_id: [0; 16],
                span_id: [0; 8],
                trace_flags: 0,
            },
        };
        self.agent.handle(event).await.unwrap();
    }
}
