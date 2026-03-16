use edgeless_function::*;

struct {{crate_name | upper_camel_case}};

edgeless_function::generate!({{crate_name | upper_camel_case}});

impl {{crate_name | upper_camel_case}}API<'_> for {{crate_name | upper_camel_case}} {
    {% if generate_http_handler -%}
    type EDGELESS_HTTP_REQUEST = edgeless_http::EdgelessHTTPRequest;
    type EDGELESS_HTTP_RESPONSE = edgeless_http::EdgelessHTTPResponse;
    {% endif -%}
    type STRING = String;

    {% if generate_http_handler -%}
    fn handle_call_http_request(_src: InstanceId, req: Self::EDGELESS_HTTP_REQUEST) -> Self::EDGELESS_HTTP_RESPONSE {
        edgeless_http::EdgelessHTTPResponse {
            status: 404,
            body: Some(Vec::<u8>::from("Not Found")),
            headers: std::collections::HashMap::<String, String>::new(),
        }
    }
    {% endif -%}

    {% assign inputs = cast_inputs | split: "," -%}
    {%- for input in inputs %}
    fn handle_cast_{{input}}(_src: InstanceId, message: String) {
        log::info!("Got Input {{input}}.");
    }
    {% endfor %}


    fn handle_internal(_data: &[u8]) {
        log::info!("Internal Message.");
        {% assign outputs = cast_outputs | split: "," -%}
        {%- for output in outputs %}
        cast_{{output}}(&"Test".to_string());
        {%- endfor %}
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Started.");
        // Triggers handle_internal.
        delayed_cast(1000, "self", &1u64.to_le_bytes());
    }

    fn handle_stop() {
        log::info!("Stopped.");
    }
}
