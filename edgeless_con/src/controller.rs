// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod client;
pub mod server;
// TODO Split and fix
// #[cfg(test)]
// pub mod test;
//
pub mod image_repository;

pub struct Controller {
    sender: futures::channel::mpsc::UnboundedSender<ControllerRequest>,
    image_repository: image_repository::ImageRepository,
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
            Self::new(Some(tp), controller_settings.placement_strategy)
        } else {
            Self::new(None, controller_settings.placement_strategy)
        }
    }

    fn new(
        telemetry_provider: Option<Box<dyn crate::ir::TelemetryProvider>>,
        placement_strategy: String,
    ) -> (Self, std::pin::Pin<Box<dyn futures::Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let image_repository = image_repository::ImageRepository::new();

        let image_repository_clone = image_repository.clone();
        let main_task = Box::pin(async move {
            match placement_strategy.as_str() {
                "weighted_random" => {
                    let mut controller_task = server::ControllerTask::<
                        crate::ir::transformations::placement::strategy::weighted_random::WeightedRandom,
                    >::new(uuid::Uuid::new_v4(), receiver, telemetry_provider, image_repository_clone);
                    controller_task.run().await;
                }
                "round_robin" => {
                    let mut controller_task = server::ControllerTask::<crate::ir::transformations::placement::strategy::round_robin::RoundRobin>::new(
                        uuid::Uuid::new_v4(),
                        receiver,
                        telemetry_provider,
                        image_repository_clone,
                    );
                    controller_task.run().await;
                }
                "random" => {
                    let mut controller_task = server::ControllerTask::<crate::ir::transformations::placement::strategy::random::Random>::new(
                        uuid::Uuid::new_v4(),
                        receiver,
                        telemetry_provider,
                        image_repository_clone,
                    );
                    controller_task.run().await;
                }
                _ => {
                    panic!("Unknown Orchestration Strategy")
                }
            };
        });

        (Controller { sender, image_repository }, main_task)
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::controller::ControllerAPI + Send> {
        client::ControllerClient::new(self.sender.clone(), self.image_repository.clone())
    }
}
