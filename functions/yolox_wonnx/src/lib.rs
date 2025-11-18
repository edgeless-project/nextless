// SPDX-FileCopyrightText: © 2021 Xavier Tao, Tommy van der Vorst & WONNX contributors
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
// YoloX WONNX Example adapted from Kazuki Ikemori's fork of WONNX https://github.com/kadu-v/wonnx/blob/dev/slice/wonnx/examples/yolox_nano.rs.
// The changes in the fork were offered to WONNX by Kazuki Ikemori: https://github.com/webonnx/wonnx/pull/214.

use edgeless_function::*;
use edgeless_function_gpu::wgpu;
use itertools::Itertools;
use wgpu::custom::InstanceInterface;

mod yolox_helpers;

struct YoloXTest;

edgeless_function::generate!(YoloXTest);

static SESSION: std::sync::OnceLock<std::sync::Arc<std::sync::Mutex<wonnx::Session>>> = std::sync::OnceLock::new();

impl YoloxNanoAPI<'_> for YoloXTest {
    type EFT_VISION_RAWIMAGE = edgeless_function_types::vision::RawImage;
    type STRING = String;

    fn handle_cast_image(_src: InstanceId, input_image: edgeless_function_types::vision::RawImage) {
        log::info!("YoloX Actor Got Image");

        #[allow(unused_mut)]
        let mut resized_image = yolox_helpers::resize_image(input_image.0);
        let converted_image = yolox_helpers::convert_image_to_net_input(&resized_image);

        let mut input_data = std::collections::HashMap::new();
        let images = converted_image.as_slice().try_into().unwrap();
        input_data.insert("images".to_string(), images);

        let result = pollster::block_on(SESSION.get().unwrap().lock().unwrap().run(&input_data)).unwrap();
        let output = result.get("output").unwrap();
        let output = output.try_into().unwrap();

        let detections = yolox_helpers::post_process(output);

        // We need a special "detections" type
        // https://stackoverflow.com/questions/56033289/join-iterator-of-str
        let detection_string: String = detections.iter().map(|d| format!("{}:{}", d.0, d.1)).join(";");

        cast_detection(&detection_string);

        // Disabled as imageproc requires bindgen
        // yolox_helpers::draw_detections(&mut resized_image, detections);
        // cast_annotated_image(&edgeless_function_types::vision::RawImage(resized_image));
    }

    fn handle_internal(_data: &[u8]) {}

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();

        log::info!("YoloX Actor Initialized");

        let custom_gpu_backend = edgeless_function_gpu::EdgeGpuInstance::new(&wgpu::InstanceDescriptor::default());
        let custom_gpu_instance = wgpu::Instance::from_custom(custom_gpu_backend);

        let model = include_bytes!("../data/yolox_nano.onnx");
        let mut session_config = wonnx::SessionConfig::new();
        session_config.wgpu_instance = Some(custom_gpu_instance);
        let session = pollster::block_on(wonnx::Session::from_bytes_with_config(model, &session_config)).unwrap();
        SESSION
            .set(std::sync::Arc::new(std::sync::Mutex::new(session)))
            .map_err(|_| panic!("Setting Session Failed"))
            .unwrap();
    }

    fn handle_stop() {
        log::info!("YoloX Actor Stopped");
    }
}
