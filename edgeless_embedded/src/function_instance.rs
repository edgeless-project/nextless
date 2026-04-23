// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub trait FunctionInstanceAPI {
    async fn start_function(
        &mut self,
        instance_specification: edgeless_api_core::function_instance::EncodedFunctionInstanceSpecification<'_>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse>;
    async fn stop_function(
        &mut self,
        function_id: edgeless_api_core::instance_id::InstanceId,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse>;
    async fn patch_function(
        &mut self,
        patch_reg: edgeless_api_core::resource_configuration::EncodedPatchRequest,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse>;
}
