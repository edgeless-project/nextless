// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use super::helpers;

#[async_trait::async_trait]
impl crate::resource_configuration::ResourceConfigurationAPI<edgeless_api_core::instance_id::InstanceId> for super::CoapClient {
    async fn start(
        &mut self,
        instance_specification: crate::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::common::StartComponentResponse<edgeless_api_core::instance_id::InstanceId>> {
        let outputs = super::helpers::std_outputs_to_core_ouputs(&instance_specification.output_mapping)?;
        let mut configuration = heapless::Vec::<(&str, &str), 16>::new();

        for (key, val) in &instance_specification.configuration {
            configuration
                .push((key, val))
                .map_err(|_| anyhow::anyhow!("Too many configuration options"))?;
        }

        let encoded_resource_spec = edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification {
            instance_id: instance_specification.resource_id,
            class_type: &instance_specification.class_type,
            output_mapping: outputs,
            configuration,
        };

        let res = self
            .call_with_reply(|token, addr, buffer| {
                edgeless_api_core::coap_mapping::COAPEncoder::encode_start_resource(addr, encoded_resource_spec.clone(), token, &mut buffer[..])
            })
            .await;

        match res {
            Ok(_) => Ok(crate::common::StartComponentResponse::InstanceId(
                instance_specification.resource_id, // edgeless_api_core::coap_mapping::CoapDecoder::decode_instance_id(&data).unwrap(),
            )),
            Err(data) => Ok(crate::common::StartComponentResponse::ResponseError(crate::common::ResponseError {
                summary: minicbor::decode::<&str>(&data).unwrap().to_string(),
                detail: None,
            })),
        }
    }

    async fn stop(&mut self, resource_id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        let res = self
            .call_with_reply(|token, addr, buffer| {
                edgeless_api_core::coap_mapping::COAPEncoder::encode_stop_resource(addr, resource_id, token, &mut buffer[..])
            })
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(data) => Err(anyhow::anyhow!(core::str::from_utf8(&data).unwrap().to_string())),
        }
    }

    async fn patch(&mut self, update: crate::common::PatchRequest) -> anyhow::Result<()> {
        let outputs = helpers::std_outputs_to_core_ouputs(&update.output_mapping)?;

        let encoded_patch_req = edgeless_api_core::resource_configuration::EncodedPatchRequest {
            instance_id: update.function_id,
            output_mapping: outputs,
        };

        let res = self
            .call_with_reply(|token, addr, buffer| {
                edgeless_api_core::coap_mapping::COAPEncoder::encode_resource_patch_request(addr, encoded_patch_req.clone(), token, &mut buffer[..])
            })
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(data) => Err(anyhow::anyhow!(core::str::from_utf8(&data).unwrap().to_string())),
        }
    }
}
