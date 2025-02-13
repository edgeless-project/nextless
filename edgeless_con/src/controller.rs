// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod client;
pub mod server;
// TODO Split and fix
// #[cfg(test)]
// pub mod test;

pub struct Controller {
    sender: futures::channel::mpsc::UnboundedSender<ControllerRequest>,
}

pub(crate) enum ControllerRequest {
    START(
        edgeless_api::workflow_instance::SpawnWorkflowRequest,
        // Reply Channel
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse>>,
    ),
    STOP(edgeless_api::workflow_instance::WorkflowId),
    LIST(
        edgeless_api::workflow_instance::WorkflowId,
        // Reply Channel
        tokio::sync::oneshot::Sender<anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowInstance>>>,
    ),
    PATCH(edgeless_api::common::PatchRequest),
    UPDATENODE(
        edgeless_api::node_registration::UpdateNodeRequest,
        // Reply Channel
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse>>,
    ),
}

#[derive(Clone)]
enum ComponentType {
    Function,
    Resource,
    SubFlow,
}

impl Controller {
    pub async fn new_from_config(
        controller_settings: crate::EdgelessConSettings,
    ) -> (Self, std::pin::Pin<Box<dyn futures::Future<Output = ()> + Send>>) {
        if let Some(prometheus_url) = &controller_settings.prometheus_url {
            let tp = Box::new(crate::prometheus_telemetry_provider::PrometheusTelemetryProvider::new(
                prometheus_url.clone(),
            ));
            Self::new(Some(tp))
        } else {
            Self::new(None)
        }
    }

    fn new(telemetry_provider: Option<Box<dyn crate::ir::TelemetryProvider>>) -> (Self, std::pin::Pin<Box<dyn futures::Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        let main_task = Box::pin(async move {
            let mut controller_task = server::ControllerTask::new(uuid::Uuid::new_v4(), receiver, telemetry_provider);
            controller_task.run().await;
        });

        (Controller { sender }, main_task)
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::controller::ControllerAPI + Send> {
        client::ControllerClient::new(self.sender.clone())
    }
}
