// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

// This is shared with the renderer. We should move this to a crate.

pub struct PongRenderRequest {
    pub paddle_1_y: u64,
    pub paddle_2_y: u64,
    pub ball_x: u64,
    pub ball_y: u64,
    pub size_y: u64,
    pub size_x: u64,
    pub points_1: u64,
    pub points_2: u64,
}

impl<'a> edgeless_function_core::Serialize<'a> for PongRenderRequest {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        let mut out = [0u8; 8 * 8];

        out[..8].copy_from_slice(&self.paddle_1_y.to_le_bytes());
        out[8..16].copy_from_slice(&self.paddle_2_y.to_le_bytes());
        out[16..24].copy_from_slice(&self.ball_x.to_le_bytes());
        out[24..32].copy_from_slice(&self.ball_y.to_le_bytes());
        out[32..40].copy_from_slice(&self.size_y.to_le_bytes());
        out[40..48].copy_from_slice(&self.size_x.to_le_bytes());
        out[48..56].copy_from_slice(&self.points_1.to_le_bytes());
        out[56..64].copy_from_slice(&self.points_2.to_le_bytes());

        out
    }
}

impl<'a> edgeless_function_core::Deserialize<'a> for PongRenderRequest {
    fn deserialize(raw: &'a [u8]) -> Self {
        let paddle_1_y = u64::from_le_bytes(raw[..8].try_into().unwrap());
        let paddle_2_y = u64::from_le_bytes(raw[8..16].try_into().unwrap());
        let ball_x = u64::from_le_bytes(raw[16..24].try_into().unwrap());
        let ball_y = u64::from_le_bytes(raw[24..32].try_into().unwrap());
        let size_y = u64::from_le_bytes(raw[32..40].try_into().unwrap());
        let size_x = u64::from_le_bytes(raw[40..48].try_into().unwrap());
        let points_1 = u64::from_le_bytes(raw[48..56].try_into().unwrap());
        let points_2 = u64::from_le_bytes(raw[56..64].try_into().unwrap());
        Self {
            paddle_1_y,
            paddle_2_y,
            ball_x,
            ball_y,
            size_y,
            size_x,
            points_1,
            points_2,
        }
    }
}
