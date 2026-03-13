// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

// https://docs.rs/embedded-graphics/latest/embedded_graphics/index.html
// https://github.com/EmbersArc/rpi_led_panel/blob/main/examples/drawing.rs
// https://docs.rs/embedded-graphics/latest/embedded_graphics/primitives/rectangle/struct.Rectangle.html
// https://docs.rs/embedded-graphics/latest/embedded_graphics/text/index.html

pub mod configuration;
pub mod messages;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::prelude::*;

use edgeless_function::*;

struct PongRenderer;

edgeless_function::generate!(PongRenderer);

const GAME_COLOR: embedded_graphics::pixelcolor::Rgb888 = embedded_graphics::pixelcolor::Rgb888::CSS_BLUE_VIOLET;

static CONFIGURATION: std::sync::OnceLock<configuration::Configuration> = std::sync::OnceLock::new();

impl PongRendererAPI<'_> for PongRenderer {
    type EFT_LED_MATRIX_MATRIX_FRAME = edgeless_function_types::led_matrix::MatrixFrame;
    type PONG_RENDER_REQUEST = messages::PongRenderRequest;

    fn handle_cast_render(_src: InstanceId, render_request: messages::PongRenderRequest) {
        let mut frame = edgeless_function_types::led_matrix::MatrixFrame::new();
        frame.0.clear(embedded_graphics::pixelcolor::Rgb888::BLACK);

        let style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
            .stroke_color(GAME_COLOR)
            .stroke_width(1)
            .fill_color(GAME_COLOR)
            .build();

        // Left Paddle
        if CONFIGURATION.get().unwrap().position_x == 0 {
            embedded_graphics::primitives::Rectangle::new(
                embedded_graphics::prelude::Point::new(
                    0,
                    (render_request.paddle_1_y as i64 - CONFIGURATION.get().unwrap().position_y as i64) as i32,
                ),
                embedded_graphics::prelude::Size::new(4, 12),
            )
            .into_styled(style)
            .draw(&mut frame.0)
            .unwrap();
        }

        // Right Paddle
        if CONFIGURATION.get().unwrap().position_x + 64 == render_request.size_x {
            embedded_graphics::primitives::Rectangle::new(
                embedded_graphics::prelude::Point::new(
                    64 - 4,
                    (render_request.paddle_2_y as i64 - CONFIGURATION.get().unwrap().position_y as i64) as i32,
                ),
                embedded_graphics::prelude::Size::new(4, 12),
            )
            .into_styled(style)
            .draw(&mut frame.0)
            .unwrap();
        }

        // Ball
        embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::prelude::Point::new(
                (render_request.ball_x as i64 - CONFIGURATION.get().unwrap().position_x as i64) as i32,
                (render_request.ball_y as i64 - CONFIGURATION.get().unwrap().position_y as i64) as i32,
            ),
            embedded_graphics::prelude::Size::new(4, 4),
        )
        .into_styled(style)
        .draw(&mut frame.0)
        .unwrap();

        // Score
        if CONFIGURATION.get().unwrap().position_y == 0 {
            let text_style = embedded_graphics::mono_font::MonoTextStyle::new(&embedded_graphics::mono_font::ascii::FONT_6X10, GAME_COLOR);

            let middle = render_request.size_x as i32 / 2 - 1;

            embedded_graphics::text::Text::new(
                &format!("{}", render_request.points_1),
                Point::new(middle - CONFIGURATION.get().unwrap().position_x as i32 - 8, 7),
                text_style,
            )
            .draw(&mut frame.0)
            .unwrap();
            embedded_graphics::text::Text::new(
                &format!("{}", render_request.points_2),
                Point::new(middle - CONFIGURATION.get().unwrap().position_x as i32 + 2, 7),
                text_style,
            )
            .draw(&mut frame.0)
            .unwrap();
        }

        cast_drawable(&frame);
    }

    fn handle_internal(data: &[u8]) {}

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Started.");

        let payload_str = payload.map(|p| str::from_utf8(p).unwrap()).unwrap_or("");
        let config = configuration::Configuration::parse(payload_str);
        log::info!("Config: {config:?}.");

        CONFIGURATION.set(config).unwrap();
    }

    fn handle_stop() {
        log::info!("Stopped.");
    }
}
