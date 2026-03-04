// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

// https://github.com/embedded-graphics/examples/blob/main/eg-0.8/examples/demo-analog-clock.rs

use embedded_graphics::prelude::*;

pub(crate) struct MockDisplay {
    sender: std::sync::mpsc::Sender<edgeless_function_types::led_matrix::MatrixFrame>,
}

impl MockDisplay {
    pub(crate) fn new(sender: std::sync::mpsc::Sender<edgeless_function_types::led_matrix::MatrixFrame>) -> Self {
        Self { sender: sender }
    }
}

impl super::Display for MockDisplay {
    fn update(&mut self, frame: edgeless_function_types::led_matrix::MatrixFrame) {
        self.sender.send(frame);
    }
}

// This has to run on the main thread and is called from fn main!
pub fn run_simulator_display(receiver: std::sync::mpsc::Receiver<edgeless_function_types::led_matrix::MatrixFrame>, node_id: uuid::Uuid) {
    let output_settings = embedded_graphics_simulator::OutputSettingsBuilder::new().scale(10).build();
    let mut display = embedded_graphics_simulator::SimulatorDisplay::<embedded_graphics::pixelcolor::Rgb888>::new(Size::new(64, 64));
    let mut window = embedded_graphics_simulator::Window::new(&format!("Node Display {}", node_id), &output_settings);

    display.clear(embedded_graphics::pixelcolor::Rgb888::BLACK);

    let mut last_frame = edgeless_function_types::led_matrix::MatrixFrame::new();

    loop {
        display.clear(embedded_graphics::pixelcolor::Rgb888::BLACK);
        if let Ok(frame) = receiver.recv_timeout(std::time::Duration::from_millis(50)) {
            last_frame = frame;
        }

        // There might be a better way to do this.
        last_frame.0.as_image().draw(&mut display).unwrap();
        window.update(&display);
        if window.events().any(|e| e == embedded_graphics_simulator::SimulatorEvent::Quit) {
            return;
        }
    }
}
