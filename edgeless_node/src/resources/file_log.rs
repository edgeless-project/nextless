// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use crate::dataplane::core::Message;
use opentelemetry::trace::TraceContextExt;
use std::io::prelude::*;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Clone)]
pub struct FileLogResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<FileLogResourceProviderInner>>,
}

struct FileLogResourceProviderInner {
    #[allow(unused)]
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: crate::dataplane::handle::DataplaneProvider,
    instances: std::collections::HashMap<edgeless_api::function_instance::InstanceId, FileLogResource>,
}

pub struct FileLogResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for FileLogResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl FileLogResource {
    async fn new(dataplane_handle: crate::dataplane::handle::DataplaneHandle, filename: &str, add_timestamp: bool) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;

        let mut outfile = std::fs::OpenOptions::new().create(true).append(true).open(filename)?;

        tracing::info!("FileLogResource created, writing to file: {filename}");

        let handle = tokio::spawn(async move {
            loop {
                let crate::dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                    target_port,
                    context,
                    ..
                } = dataplane_handle.receive_next().await;

                // TODO Properly handle the async parts here.
                // This is best done after creating a resource abstraction.
                let s = tracing::trace_span!("file_log", el_span_kind = "sink_invocation");

                if !s.is_disabled() {
                    let parent_context = if context.is_valid() {
                        opentelemetry::Context::new().with_remote_span_context(context)
                    } else {
                        opentelemetry::Context::new()
                    };
                    s.set_parent(parent_context).unwrap();
                }

                let _ = s.entered();
                let mut need_reply = false;
                let message_data = match message {
                    Message::Call(data) => {
                        need_reply = true;
                        String::from_utf8(data).unwrap()
                    }
                    Message::Cast(data) => String::from_utf8(data).unwrap(),
                    _ => {
                        continue;
                    }
                };

                if target_port != edgeless_api::function_instance::PortId("line".to_string()) {
                    tracing::debug!("FileLog: Bad Port {target_port:?}");
                    continue;
                }

                let line = match add_timestamp {
                    true => format!("{} {}", chrono::Utc::now().to_rfc3339(), message_data),
                    false => message_data,
                };

                if let Err(e) = writeln!(outfile, "{line}") {
                    tracing::error!("Could not write to file the message '{line}': {e}");
                }

                if need_reply {
                    dataplane_handle
                        .reply(source_id, channel_id, crate::dataplane::core::CallRet::Reply(Vec::new()))
                        .await;
                }
            }
        });

        Ok(Self { join_handle: handle })
    }
}

impl FileLogResourceProvider {
    pub async fn new(
        dataplane_provider: crate::dataplane::handle::DataplaneProvider,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(FileLogResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                instances: std::collections::HashMap::<edgeless_api::function_instance::InstanceId, FileLogResource>::new(),
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for FileLogResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        if let Some(filename) = instance_specification.configuration.get("filename") {
            let mut lck = self.inner.lock().await;

            let dataplane_handle = lck.dataplane_provider.get_handle_for(instance_specification.resource_id, None).await;

            match FileLogResource::new(
                dataplane_handle,
                filename,
                instance_specification.configuration.contains_key("add-timestamp"),
            )
            .await
            {
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
        Ok(edgeless_api::common::StartComponentResponse::ResponseError(
            edgeless_api::common::ResponseError {
                summary: "Invalid resource configuration".to_string(),
                detail: Some("Field 'filename' missing".to_string()),
            },
        ))
    }

    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.inner.lock().await.instances.remove(&resource_id);
        Ok(())
    }

    async fn patch(&mut self, _update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        // the resource has no channels: nothing to be patched
        Ok(())
    }
}
