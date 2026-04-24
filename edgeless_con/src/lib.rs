// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

//! Nextless Controller
//!
//! This crate is split into the service-related parts (module [controller]) and the model and transformation engine (module [ir]).
//!
//! The compiler-inspired aspects can be found in the [ir] module.

pub mod controller;
pub mod ir;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessConSettings {
    #[serde(alias = "controller_url")]
    pub controller_grpc_listen_url: String,
    pub controller_coap_listen_url: Option<String>,
    pub prometheus_url: Option<String>,
    pub placement_strategy: String,
    pub opentelemetry_export: Option<OpenTelemetryExportConfig>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct OpenTelemetryExportConfig {
    pub enabled: bool,
    pub endpoint: String,
}

pub async fn edgeless_con_main(settings: EdgelessConSettings) {
    tracing::info!("Starting Edgeless Controller");
    tracing::debug!("Settings: {settings:?}");

    let (mut controller, controller_task) = controller::Controller::new_from_config(settings.clone()).await;

    let server_task =
        edgeless_api::grpc_impl::controller::WorkflowInstanceAPIServer::run(controller.get_api_client(), settings.controller_grpc_listen_url.clone());

    if let Ok((_, ip_str, _)) = edgeless_api::util::parse_http_host(&settings.controller_grpc_listen_url) {
        if let Ok(ip) = ip_str.parse::<std::net::IpAddr>() {
            if !ip.is_loopback() {
                tracing::warn!("Security Warning: Controller gRPC server listening on IP {ip_str}. As nextless does not currently contain any security features, it should only receive traffic from fully trusted networks!")
            }
        }
    }

    let coap_server_task = if let Some(url) = settings.controller_coap_listen_url {
        if let Ok((proto, address, port)) = edgeless_api::util::parse_http_host(&url) {
            if proto != edgeless_api::util::Proto::COAP {
                tracing::warn!("Wrong protocol for the CoAP node register ({url}): assuming coap://");
            }
            if address != "0.0.0.0" {
                tracing::warn!("CoAP node register requested to be bound at {address}: ignored, using 0.0.0.0 instead");
            }
            tracing::info!("Start Controller COAP: {address}:{port}");
            edgeless_api::coap_impl::orchestration::CoapOrchestrationServer::run(
                controller.get_api_client().node_registration_api(),
                controller.get_api_client().image_repository(),
                std::net::SocketAddrV4::new("0.0.0.0".parse().unwrap(), port),
            )
        } else {
            tracing::error!("Wrong URL for the CoAP node register: {url}");
            Box::pin(async {})
        }
    } else {
        Box::pin(async {})
    };

    futures::join!(controller_task, server_task, coap_server_task);
}

pub fn edgeless_con_default_conf() -> String {
    let con_settings = EdgelessConSettings {
        controller_grpc_listen_url: "http://127.0.0.1:7001".to_string(),
        controller_coap_listen_url: None,
        prometheus_url: None,
        placement_strategy: "random".to_string(),
        opentelemetry_export: Some(OpenTelemetryExportConfig {
            enabled: false,
            endpoint: "http://localhost:4318/v1/traces".to_string(),
        }),
    };

    toml::to_string_pretty(&con_settings).expect("Could not serialize default controller settings.")
}
