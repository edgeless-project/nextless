// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct DetectionTester;

edgeless_function::generate!(DetectionTester);

impl DetectionTesterAPI<'_> for DetectionTester {
    type EFT_VISION_RAWIMAGE = edgeless_function_types::vision::RawImage;
    type STRING = String;

    fn handle_cast_detection(_src: InstanceId, detection: String) {
        log::info!("Detection Tester Got Detection: {}", detection);
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Detection Tester: Send Test Image Bird");

        let image_bytes = include_bytes!("../data/bird.jpg");
        let image_buffer: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> = image::load_from_memory(image_bytes).unwrap().to_rgb8();

        cast_test_image(&edgeless_function_types::vision::RawImage(image_buffer));

        delayed_cast(1000, "self", b"wakeup");
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Detection Tester Initialized");
        delayed_cast(5000, "self", b"wakeup");
    }

    fn handle_stop() {
        log::info!("Detection Tester Stopped");
    }
}
