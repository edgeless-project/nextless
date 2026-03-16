// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

mod configuration;
mod messages;

use edgeless_function::*;

struct GameOfLifeClock;

edgeless_function::generate!(GameOfLifeClock);

static CONFIGURATION: std::sync::OnceLock<configuration::Configuration> = std::sync::OnceLock::new();

impl GameOfLifeClockAPI<'_> for GameOfLifeClock {
    type GAME_OF_LIFE_ITERATION = messages::Iteration;

    fn handle_internal(data: &[u8]) {
        let iteration = u64::from_le_bytes(data.try_into().unwrap());
        let update = messages::Iteration { iteration_id: iteration };
        cast_trigger(&update);
        delayed_cast(CONFIGURATION.get().unwrap().period_ms as u64, "self", &(iteration + 1).to_le_bytes());
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Started.");

        let payload_str = payload.map(|p| str::from_utf8(p).unwrap()).unwrap_or("");
        let config = configuration::Configuration::parse(payload_str);
        log::info!("Config: {config:?}");
        CONFIGURATION.set(config).unwrap();

        if config.period_ms > 0 {
            delayed_cast(config.period_ms as u64, "self", &1u64.to_le_bytes());
        }
    }

    fn handle_stop() {
        log::info!("Stopped.");
    }
}
