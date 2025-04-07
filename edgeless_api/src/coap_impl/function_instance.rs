// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use std::str::FromStr;

use crate::image_repository::FunctionImageHash;

#[async_trait::async_trait]
impl crate::function_instance::FunctionInstanceAPI<edgeless_api_core::instance_id::InstanceId> for super::CoapClient {
    async fn start(
        &mut self,
        spawn_request: crate::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<crate::common::StartComponentResponse<edgeless_api_core::instance_id::InstanceId>> {
        let cbor_class = edgeless_api_core::function_instance::EncodedFunctionClassSpecification {
            class_id: heapless::String::<32>::from_str(spawn_request.code.function_class_id.as_str())
                .map_err(|_| anyhow::anyhow!("String to long!"))?,
            class_type: heapless::String::<32>::from_str(spawn_request.code.function_class_type.as_str())
                .map_err(|_| anyhow::anyhow!("String to long!"))?,
            version: heapless::String::<8>::from_str(spawn_request.code.function_class_version.as_str())
                .map_err(|_| anyhow::anyhow!("String to long!"))?,
            image_size: spawn_request.code.function_class_code.len() as u64,
            image_hash: spawn_request.code.function_class_code.image_hash(),
        };
        let cbor_req = edgeless_api_core::function_instance::EncodedFunctionInstanceSpecification {
            instance_id: spawn_request.instance_id,
            class: cbor_class,
            input_mapping: heapless::Vec::new(),
            output_mapping: super::helpers::std_outputs_to_core_ouputs(&spawn_request.output_mapping)?,
        };

        let res = self
            .call_with_reply(|token, addr, buffer| {
                edgeless_api_core::coap_mapping::COAPEncoder::encode_start_function(addr, cbor_req.clone(), token, &mut buffer[..])
            })
            .await;

        match res {
            Ok(_data) => Ok(crate::common::StartComponentResponse::InstanceId(spawn_request.instance_id)),
            Err(data) => Ok(crate::common::StartComponentResponse::ResponseError(crate::common::ResponseError {
                summary: minicbor::decode::<&str>(&data).unwrap().to_string(),
                detail: None,
            })),
        }
    }
    async fn stop(&mut self, function_id: edgeless_api_core::instance_id::InstanceId) -> anyhow::Result<()> {
        let res = self
            .call_with_reply(|token, addr, buffer| {
                edgeless_api_core::coap_mapping::COAPEncoder::encode_stop_function(addr, function_id, token, &mut buffer[..])
            })
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(data) => Err(anyhow::anyhow!(core::str::from_utf8(&data).unwrap().to_string())),
        }
    }
    async fn patch(&mut self, update: crate::common::PatchRequest) -> anyhow::Result<()> {
        let outputs = super::helpers::std_outputs_to_core_ouputs(&update.output_mapping)?;

        let encoded_patch_req = edgeless_api_core::resource_configuration::EncodedPatchRequest {
            instance_id: update.function_id,
            output_mapping: outputs,
        };

        let res = self
            .call_with_reply(|token, addr, buffer| {
                edgeless_api_core::coap_mapping::COAPEncoder::encode_function_patch_request(addr, encoded_patch_req.clone(), token, &mut buffer[..])
            })
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(data) => Err(anyhow::anyhow!(core::str::from_utf8(&data).unwrap().to_string())),
        }
    }
}
