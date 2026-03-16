// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

// Extracted from ./functions/game_of_life/src/configuration.rs

#[derive(Debug, Clone, Copy)]
pub struct Configuration {
    pub period_ms: usize,
}

impl Configuration {
    pub fn parse(payload: &str) -> Self {
        let kvs = payload.split(",");
        let mut period_ms = 1000;

        for kv in kvs {
            if let Some((k, v)) = kv.split_once("=") {
                match k {
                    "period_ms" => {
                        if let Ok(v) = v.parse() {
                            period_ms = v;
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

        Self { period_ms }
    }
}
