// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("controller.toml"))]
    config_file: String,
    #[arg(short, long, default_value_t = String::from(""))]
    template: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if !args.template.is_empty() {
        edgeless_api::util::create_template(&args.template, edgeless_con::edgeless_con_default_conf().as_str())?;
        return Ok(());
    }
    let conf: edgeless_con::EdgelessConSettings = toml::from_str(&std::fs::read_to_string(args.config_file)?)?;

    setup_tracing(&conf.opentelemetry_export);

    let async_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build()?;
    let async_tasks = vec![async_runtime.spawn(edgeless_con::edgeless_con_main(conf.clone()))];

    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });

    Ok(())
}

fn setup_tracing(otel_export_config: &Option<edgeless_con::OpenTelemetryExportConfig>) {
    if let Some(otel_export_config) = otel_export_config {
        if otel_export_config.enabled {
            println!("Setup with OpenTelemetry, endpoint: {}", otel_export_config.endpoint);
            setup_tracing_with_opentelemetry(otel_export_config.endpoint.clone());
            return;
        }
    }

    println!("Setup with basic fmt tracing");
    tracing_subscriber::registry().with(get_fmt_layer()).init();
}

#[cfg(feature = "otel")]
fn setup_tracing_with_opentelemetry(otlp_endpoint: String) {
    // https://broch.tech/posts/rust-tracing-opentelemetry/
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_otlp::WithExportConfig;

    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
        .with_endpoint(otlp_endpoint)
        .build()
        .unwrap();
    let otel_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(otlp_exporter)
        .with_resource(opentelemetry_sdk::Resource::builder().with_service_name("edgeless_controller").build())
        .build();

    let tracer = otel_provider.tracer("edgeless_controller");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry().with(get_fmt_layer()).with(otel_layer).init();
}

#[cfg(not(feature = "otel"))]
fn setup_tracing_with_opentelemetry(_otlp_endpoint: String) {
    panic!("Config requests OpenTelemetry export but controller has been built without support for OpenTelemetry.")
}

// https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/#runtime-configuration-with-layers
fn get_fmt_layer<S>() -> Box<dyn Layer<S> + Send + Sync + 'static>
where
    S: tracing::Subscriber,
    for<'a> S: tracing_subscriber::registry::LookupSpan<'a>,
{
    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive("edgeless=info".parse().unwrap())
        .from_env()
        .expect("Bad RUST_LOG value");
    tracing_subscriber::fmt::layer().with_filter(env_filter).boxed()
}
