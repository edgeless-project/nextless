use opentelemetry::trace::Span;
use opentelemetry::trace::{TraceContextExt, Tracer};

pub struct TracingContext {
    pub parent_context: opentelemetry::Context,
}

pub(crate) fn span(
    span_id: String,
    parent: opentelemetry::trace::SpanContext,
    input_port: Option<edgeless_api::function_instance::PortId>,
) -> opentelemetry::global::BoxedSpan {
    let context = opentelemetry::Context::current();
    let mut span = if parent.is_valid() {
        assert!(parent.is_sampled());
        let context = context.with_remote_span_context(parent);
        context.span().add_event("msg_received", Vec::new());
        context.span().end();
        opentelemetry::global::tracer("actor_runtime").start_with_context(span_id, &context)
    } else {
        // assert!(false);
        opentelemetry::global::tracer("actor_runtime").start(span_id)
    };
    if let Some(input_port) = &input_port {
        span.set_attribute(opentelemetry::KeyValue::new("component.input_port", input_port.0.clone()));
    }
    // span.set_attribute(opentelemetry::KeyValue::new("actor.id", "example2"));
    span
}
