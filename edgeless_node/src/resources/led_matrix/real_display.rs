// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::image::ImageDrawable;

pub struct RealDisplay {
    frame: std::sync::Arc<std::sync::Mutex<Option<edgeless_function_types::led_matrix::MatrixFrame>>>,
}

impl super::Display for RealDisplay {
    fn update(&mut self, frame: edgeless_function_types::led_matrix::MatrixFrame) {
        *self.frame.lock().unwrap() = Some(frame);
    }
}

impl RealDisplay {
    pub fn new() -> Self {
        let frame_var = std::sync::Arc::new(std::sync::Mutex::new(None));
        let cloned_frame_var = frame_var.clone();

        std::thread::spawn(move || {
            matrix_task(cloned_frame_var);
        });

        Self { frame: frame_var }
    }
}

// The library requires a constant stream of frames, not just real updates.
// Display handling: cf. https://github.com/EmbersArc/rpi_led_panel/blob/main/examples/drawing.rs
fn matrix_task(shared_frame: std::sync::Arc<std::sync::Mutex<Option<edgeless_function_types::led_matrix::MatrixFrame>>>) {
    let mut config: rpi_led_panel::RGBMatrixConfig = rpi_led_panel::RGBMatrixConfig::default();
    // The default is different from the argh default used in the examples.
    config.hardware_mapping = rpi_led_panel::HardwareMapping::regular();

    let (mut matrix, mut canvas) = rpi_led_panel::RGBMatrix::new(config, 0).unwrap();

    let mut frame = edgeless_function_types::led_matrix::MatrixFrame::new();

    frame.0.clear(embedded_graphics::pixelcolor::Rgb888::new(128, 0, 0));

    loop {
        let new_frame = shared_frame.lock().unwrap().take();

        if let Some(new_frame) = new_frame {
            frame = new_frame;
        }

        // There might be a better way to do this.
        frame.0.as_image().draw(canvas.as_mut()).unwrap();

        canvas = matrix.update_on_vsync(canvas);
    }
}
