// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use embedded_graphics::image::GetPixel;

pub(crate) struct MockDisplay {}

impl super::Display for MockDisplay {
    fn update(&mut self, frame: edgeless_function_types::led_matrix::MatrixFrame) {
        let mut out = "Sampling top-right 10x10 corner:\n".to_string();

        for row in 0..10 {
            for col in 0..10 {
                out.push_str(&format!(
                    "{:?}",
                    frame.0.pixel(embedded_graphics::prelude::Point { x: col, y: row }).unwrap()
                ));
            }
            out.push_str("\n");
        }

        print!("{}\n", out);
    }
}
