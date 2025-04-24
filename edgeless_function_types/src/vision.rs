// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct RawImage(pub image::ImageBuffer<image::Rgb<u8>, Vec<u8>>);

impl<'a> edgeless_function_core::Serialize<'a> for RawImage {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        let mut out = std::io::Cursor::new(Vec::new());
        self.0.write_to(&mut out, image::ImageFormat::Jpeg).unwrap();

        out.into_inner()
    }
}

impl<'a> edgeless_function_core::Deserialize<'a> for RawImage {
    fn deserialize(raw: &'a [u8]) -> Self {
        RawImage(image::load_from_memory(raw).unwrap().to_rgb8())
    }
}
