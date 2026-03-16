// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

// Wraps an embedded graphics framebuffer compatible with https://github.com/EmbersArc/rpi_led_panel
pub struct MatrixFrame(
    pub  embedded_graphics::framebuffer::Framebuffer<
        embedded_graphics::pixelcolor::Rgb888,
        <embedded_graphics::pixelcolor::Rgb888 as embedded_graphics::pixelcolor::PixelColor>::Raw,
        embedded_graphics::pixelcolor::raw::LittleEndian,
        64,
        64,
        { embedded_graphics::framebuffer::buffer_size::<embedded_graphics::pixelcolor::Rgb888>(64, 64) },
    >,
);

impl MatrixFrame {
    pub fn new() -> Self {
        MatrixFrame(embedded_graphics::framebuffer::Framebuffer::new())
    }
}

impl<'a> edgeless_function_core::Serialize<'a> for MatrixFrame {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        self.0.data()
    }
}

impl<'a> edgeless_function_core::Deserialize<'a> for MatrixFrame {
    fn deserialize(raw: &'a [u8]) -> Self {
        let mut frame = MatrixFrame::new();
        frame.0.data_mut().copy_from_slice(raw);

        frame
    }
}
