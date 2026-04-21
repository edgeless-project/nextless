// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, Copy)]
pub struct Configuration {
    pub position_y: usize,
    pub position_x: usize,
    pub draw_border: bool,
    pub corner_blocks: bool,
    pub periodic_glider: bool,
    pub periodic_noise: bool,
}

impl Configuration {
    pub fn parse(payload: &str) -> Self {
        let kvs = payload.split(",");

        let mut position_y = 0;
        let mut position_x = 0;
        let mut draw_border = false;
        let mut corner_blocks = false;
        let mut periodic_glider = false;
        let mut periodic_noise = false;

        for kv in kvs {
            if let Some((k, v)) = kv.split_once("=") {
                match k {
                    "position_y" => {
                        if let Ok(v) = v.parse() {
                            position_y = v;
                        } else {
                            log::warn!("Bad Configuration");
                        }
                    }
                    "position_x" => {
                        if let Ok(v) = v.parse() {
                            position_x = v;
                        } else {
                            log::warn!("Bad Configuration");
                        }
                    }
                    "draw_border" => {
                        if let Ok(v) = v.parse() {
                            draw_border = v;
                        } else {
                            log::warn!("Bad Configuration");
                        }
                    }
                    "corner_blocks" => {
                        if let Ok(v) = v.parse() {
                            corner_blocks = v;
                        } else {
                            log::warn!("Bad Configuration");
                        }
                    }
                    "periodic_glider" => {
                        if let Ok(v) = v.parse() {
                            periodic_glider = v;
                        } else {
                            log::warn!("Bad Configuration");
                        }
                    }
                    "periodic_noise" => {
                        if let Ok(v) = v.parse() {
                            periodic_noise = v;
                        } else {
                            log::warn!("Bad Configuration");
                        }
                    }
                    _ => {
                        log::info!("Unknown Configuration Key");
                    }
                }
            }
        }

        Self {
            position_y,
            position_x,
            draw_border,
            corner_blocks,
            periodic_glider,
            periodic_noise,
        }
    }
}
