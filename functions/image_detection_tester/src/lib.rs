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

    fn handle_internal(data: &[u8]) {
        let bird_bytes = include_bytes!("../data/bird.jpg");
        let cat_bytes = include_bytes!("../data/cat.jpg");

        let item = core::str::from_utf8(data).unwrap();
        let image_buffer: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> = if item == "bird" {
            log::info!("Detection Tester: Send Test Image: Bird");
            image::load_from_memory(bird_bytes).unwrap().to_rgb8()
        } else {
            log::info!("Detection Tester: Send Test Image: Cat");
            image::load_from_memory(cat_bytes).unwrap().to_rgb8()
        };

        cast_test_image(&edgeless_function_types::vision::RawImage(image_buffer));

        if item == "bird" {
            delayed_cast(1000, "self", b"cat");
        } else {
            delayed_cast(1000, "self", b"bird");
        }
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Detection Tester Initialized");
        delayed_cast(5000, "self", b"bird");
    }

    fn handle_stop() {
        log::info!("Detection Tester Stopped");
    }
}
