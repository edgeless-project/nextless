// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_function::*;
use image::GenericImage;

struct ImageScaler;

edgeless_function::generate!(ImageScaler);

impl ImageScalerAPI<'_> for ImageScaler {
    type EFT_VISION_RAWIMAGE = edgeless_function_types::vision::RawImage;

    fn handle_cast_unscaled(_src: InstanceId, unscaled: edgeless_function_types::vision::RawImage) {
        let img = resize_image(&unscaled.0);

        let wrapped = edgeless_function_types::vision::RawImage(img);
        cast_scaled(&wrapped);
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Image Scaler Wakeup");
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Image Scaler Started");
    }

    fn handle_stop() {
        log::info!("Image Scaler Stopped");
    }
}

fn resize_image(img: &image::RgbImage) -> image::RgbImage {
    let resized = image::imageops::resize(img, 640, 640, image::imageops::FilterType::Nearest);

    if resized.height() != 640 || resized.width() != 640 {
        let mut new_img = image::RgbImage::new(640, 640);

        new_img
            .sub_image(0, 0, resized.width(), resized.height())
            .copy_from(&resized, 0, 0)
            .unwrap();

        return new_img;
    } else {
        return resized;
    }
}
