// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Debug)]
pub struct Configuration {
    /// X Position (from top-left) in Pixels
    pub position_x: u64,
    /// Y Position (from top-left) in Pixels
    pub position_y: u64,
}

impl Configuration {
    pub fn parse(payload: &str) -> Self {
        let kvs = payload.split(",");

        let mut position_y = 0;
        let mut position_x = 0;

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
                    _ => {
                        log::info!("Unknown Configuration Key");
                    }
                }
            }
        }

        Self { position_y, position_x }
    }
}
