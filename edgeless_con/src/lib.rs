// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
mod controller;
mod ir;
pub mod prometheus_telemetry_provider;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessConOrcConfig {
    pub domain_id: String,
    pub orchestrator_url: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessConSettings {
    pub controller_url: String,
    pub prometheus_url: Option<String>,
    // pub orchestrators: Vec<EdgelessConOrcConfig>,
}

pub async fn edgeless_con_main(settings: EdgelessConSettings) {
    log::info!("Starting Edgeless Controller at {}", settings.controller_url);
    log::debug!("Settings: {:?}", settings);

    let (mut controller, controller_task) = controller::Controller::new_from_config(settings.clone()).await;

    let server_task =
        edgeless_api::grpc_impl::controller::WorkflowInstanceAPIServer::run(controller.get_api_client(), settings.controller_url.clone());

    let coap_server_task = if let Some(url) = Some("coap://0.0.0.0:7001") {
        if let Ok((proto, address, port)) = edgeless_api::util::parse_http_host(&url) {
            if proto != edgeless_api::util::Proto::COAP {
                log::warn!("Wrong protocol for the CoAP node register ({}): assuming coap://", url);
            }
            if address != "0.0.0.0" {
                log::warn!("CoAP node register requested to be bound at {}: ignored, using 0.0.0.0 instead", address);
            }
            log::info!("Start Controller COAP: {}:{}", address, port);
            edgeless_api::coap_impl::orchestration::CoapOrchestrationServer::run(
                controller.get_api_client().node_registration_api(),
                controller.get_api_client().image_repository(),
                std::net::SocketAddrV4::new("0.0.0.0".parse().unwrap(), port),
            )
        } else {
            log::error!("Wrong URL for the CoAP node register: {}", url);
            Box::pin(async {})
        }
    } else {
        Box::pin(async {})
    };

    futures::join!(controller_task, server_task, coap_server_task);
}

pub fn edgeless_con_default_conf() -> String {
    String::from(
        r##"controller_url = "http://127.0.0.1:7001"
            prometheus_url = "http://127.0.0.1:9090"
"##,
    )
}
