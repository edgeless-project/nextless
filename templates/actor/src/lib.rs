{%- assign inputs = cast_inputs | split: "," -%}
{%- assign outputs = cast_outputs | split: "," -%}
use edgeless_function::*;
{% if generate_matrix_output -%}
use embedded_graphics::prelude::*;
{% endif %}
struct {{crate_name | upper_camel_case}};

edgeless_function::generate!({{crate_name | upper_camel_case}});

impl {{crate_name | upper_camel_case}}API<'_> for {{crate_name | upper_camel_case}} {
    {% if generate_http_handler -%}
    type EFT_HTTP_REQUEST = edgeless_function_types::http::EdgelessHTTPRequest;
    type EFT_HTTP_RESPONSE = edgeless_function_types::http::EdgelessHTTPResponse;
    {% endif -%}
    {% if generate_matrix_output -%}
    type EFT_LED_MATRIX_MATRIX_FRAME = edgeless_function_types::led_matrix::MatrixFrame;
    {% endif -%}
    {% if inputs != empty or outputs != empty -%}
    type STRING = String;
    {%endif -%}
    {%- if generate_http_handler %}
    fn handle_call_http_request(_src: InstanceId, req: Self::EFT_HTTP_REQUEST) -> Self::EFT_HTTP_RESPONSE {
        edgeless_function_types::http::EdgelessHTTPResponse {
            status: 404,
            body: Some(Vec::<u8>::from("Not Found")),
            headers: std::collections::HashMap::<String, String>::new(),
        }
    }
    {%- endif -%}

    {%- for input in inputs %}
    fn handle_cast_{{input}}(_src: InstanceId, message: String) {
        log::info!("Got Input {{input}}.");
    }
    {% endfor %}

    fn handle_internal(_data: &[u8]) {
        log::info!("Internal Message.");
        {%- for output in outputs %}
        cast_{{output}}(&"Test".to_string());
        {%- endfor %}
        {% if generate_matrix_output -%}
        draw_text("{{crate_name}}");
        {%- endif %}
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
{% if generate_matrix_output -%}
fn draw_text(text: &str) {
    // https://docs.rs/embedded-graphics/latest/embedded_graphics/index.html
    // https://github.com/EmbersArc/rpi_led_panel/blob/main/examples/drawing.rs
    // https://docs.rs/embedded-graphics/latest/embedded_graphics/text/index.html
    let mut frame = edgeless_function_types::led_matrix::MatrixFrame::new();
    frame.0.clear(embedded_graphics::pixelcolor::Rgb888::BLACK);

    let text_style = embedded_graphics::mono_font::MonoTextStyle::new(&embedded_graphics::mono_font::ascii::FONT_4X6, embedded_graphics::pixelcolor::Rgb888::CSS_BLUE_VIOLET);
    embedded_graphics::text::Text::new(
        &format!("{}", text),
        Point::new(1, 5),
        text_style,
    )
    .draw(&mut frame.0)
    .unwrap();

    cast_drawable(&frame);
}
{% endif -%}
