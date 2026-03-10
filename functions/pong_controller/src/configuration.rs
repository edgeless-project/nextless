// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Debug)]
pub struct Configuration {
    /// Height in Pixels
    pub size_y: u64,
    /// Lenght in Pixels
    pub size_x: u64,
}

impl Configuration {
    pub fn parse(payload: &str) -> Self {
        let kvs = payload.split(",");

        let mut size_x = 64;
        let mut size_y = 64;

        for kv in kvs {
            if let Some((k, v)) = kv.split_once("=") {
                match k {
                    "size_y" => {
                        if let Ok(v) = v.parse() {
                            size_y = v;
                        } else {
                            log::warn!("Bad Configuration");
                        }
                    }
                    "size_x" => {
                        if let Ok(v) = v.parse() {
                            size_x = v;
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

        Self { size_x, size_y }
    }
}
