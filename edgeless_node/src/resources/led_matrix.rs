// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

// Derived from the file_log resource.

mod mock_display;
#[cfg(feature = "hardware_led_matrix")]
mod real_display;
#[cfg(feature = "simulator_led_matrix")]
pub mod simulator_display;

use edgeless_function_core::Deserialize;

trait Display {
    fn update(&mut self, frame: edgeless_function_types::led_matrix::MatrixFrame);
}

#[derive(Clone)]
pub struct LedMatrixResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<LedMatrixResourceProviderInner>>,
}

struct LedMatrixResourceProviderInner {
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    sender: std::sync::mpsc::Sender<edgeless_function_types::led_matrix::MatrixFrame>,
    instances: std::collections::HashMap<edgeless_api::function_instance::InstanceId, LedMatrixResource>,
}

pub struct LedMatrixResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for LedMatrixResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl LedMatrixResource {
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        #[allow(unused)] sender: std::sync::mpsc::Sender<edgeless_function_types::led_matrix::MatrixFrame>,
    ) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;

        #[cfg(not(any(feature = "hardware_led_matrix", feature = "simulator_led_matrix")))]
        let mut display = mock_display::MockDisplay {};

        #[cfg(feature = "simulator_led_matrix")]
        let mut display = simulator_display::MockDisplay::new(sender);

        #[cfg(feature = "hardware_led_matrix")]
        let mut display = real_display::RealDisplay::new();

        tracing::info!("RGB Led Matrix Resource Created.");

        let handle = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent { message, target_port, .. } = dataplane_handle.receive_next().await;

                if &target_port.0 != "update" {
                    continue;
                }

                let edgeless_dataplane::core::Message::Cast(message_data) = message else {
                    continue;
                };

                let frame = edgeless_function_types::led_matrix::MatrixFrame::deserialize(&message_data);

                display.update(frame);
            }
        });

        Ok(Self { join_handle: handle })
    }
}

impl LedMatrixResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        _resource_provider_id: edgeless_api::function_instance::InstanceId,
        sender: std::sync::mpsc::Sender<edgeless_function_types::led_matrix::MatrixFrame>,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(LedMatrixResourceProviderInner {
                dataplane_provider,
                instances: std::collections::HashMap::<edgeless_api::function_instance::InstanceId, LedMatrixResource>::new(),
                sender,
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for LedMatrixResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let mut lck = self.inner.lock().await;

        let dataplane_handle = lck.dataplane_provider.get_handle_for(instance_specification.resource_id, None).await;

        match LedMatrixResource::new(dataplane_handle, lck.sender.clone()).await {
            Ok(resource) => {
                lck.instances.insert(instance_specification.resource_id, resource);
                return Ok(edgeless_api::common::StartComponentResponse::InstanceId(
                    instance_specification.resource_id,
                ));
            }
            Err(err) => {
                return Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                    edgeless_api::common::ResponseError {
                        summary: "Invalid resource configuration".to_string(),
                        detail: Some(err.to_string()),
                    },
                ));
            }
        }
    }

    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.inner.lock().await.instances.remove(&resource_id);
        Ok(())
    }

    async fn patch(&mut self, _update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        //No output ports that need patching
        Ok(())
    }
}
