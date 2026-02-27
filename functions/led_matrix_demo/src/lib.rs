// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

// https://docs.rs/embedded-graphics/latest/embedded_graphics/index.html
// https://github.com/EmbersArc/rpi_led_panel/blob/main/examples/drawing.rs

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::prelude::*;

use edgeless_function::*;

struct LedMatrixDemo;

edgeless_function::generate!(LedMatrixDemo);

impl LedMatrixDemoAPI<'_> for LedMatrixDemo {
    type EFT_LED_MATRIX_MATRIX_FRAME = edgeless_function_types::led_matrix::MatrixFrame;

    fn handle_internal(data: &[u8]) {
        let iteration_id = u32::from_ne_bytes(data.try_into().unwrap());

        let mut frame = edgeless_function_types::led_matrix::MatrixFrame::new();

        frame.0.clear(embedded_graphics::pixelcolor::Rgb888::BLACK);

        let mut remaining = iteration_id + 1;
        log::info!("Sending Value: {}", remaining);

        for y in 0..16 {
            let row_items = std::cmp::min(remaining, 16);
            remaining -= row_items;

            for x in 0..row_items {
                // https://docs.rs/embedded-graphics/latest/embedded_graphics/primitives/rectangle/struct.Rectangle.html
                let style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
                    .stroke_color(embedded_graphics::pixelcolor::Rgb888::WHITE)
                    .stroke_width(1)
                    .fill_color(embedded_graphics::pixelcolor::Rgb888::WHITE)
                    .build();

                embedded_graphics::primitives::Rectangle::new(
                    embedded_graphics::prelude::Point::new((x * 4) as i32, (y * 4) as i32),
                    embedded_graphics::prelude::Size::new(4, 4),
                )
                .into_styled(style)
                .draw(&mut frame.0)
                .unwrap();
            }

            if remaining <= 0 {
                break;
            }
        }

        cast_value(&frame);

        delayed_cast(100, "self", &((iteration_id + 1) % (16 * 16)).to_ne_bytes());
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Stated. Start sending in 5s");
        delayed_cast(5000, "self", &0u32.to_ne_bytes());
    }

    fn handle_stop() {
        log::info!("Actor Stopped");
    }
}
