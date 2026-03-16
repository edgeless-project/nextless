// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub mod messages;
use messages::*;

pub mod game_state;
use game_state::*;

pub mod configuration;

use core::convert::TryInto;

use edgeless_function::*;

struct GameOfLife;

edgeless_function::generate!(GameOfLife);

// Our state is not stored in a struct instance (and passed as self) due to historic reasons.
static STATE: std::sync::OnceLock<std::sync::Mutex<GameState>> = std::sync::OnceLock::new();
static CONFIGURATION: std::sync::OnceLock<configuration::Configuration> = std::sync::OnceLock::new();

impl GameOfLifeAPI<'_> for GameOfLife {
    type EFT_LED_MATRIX_MATRIX_FRAME = edgeless_function_types::led_matrix::MatrixFrame;
    type GAME_OF_LIFE_ITERATION = Iteration;
    type GAME_OF_LIFE_UPDATE_ROW = UpdateRow;
    type GAME_OF_LIFE_UPDATE_COL = UpdateCol;
    type GAME_OF_LIFE_UPDATE_CORNER = UpdateCorner;

    fn handle_cast_iteration_clock_i(_src: InstanceId, clock_update: Iteration) {
        log::debug!("Handle Clock: {}", clock_update.iteration_id);

        let mut game_state = STATE.get().unwrap().lock().unwrap();

        if CONFIGURATION.get().unwrap().periodic_glider {
            if clock_update.iteration_id % 30 == 0 {
                game_state.spawn_glider(7, 7);
            }
        }

        game_state.logic_update(clock_update.iteration_id);

        // Initialize with positions
        if clock_update.iteration_id == 1 {
            game_state.spawn_char((48 + CONFIGURATION.get().unwrap().position_y) as u8 as char, 0, 0);
            game_state.spawn_char((48 + CONFIGURATION.get().unwrap().position_x) as u8 as char, 8, 8);
        }

        let update = game_state.draw();

        game_state.error_occured = None;

        Self::send_left_update(clock_update.iteration_id, &mut game_state);
        Self::send_right_update(clock_update.iteration_id, &mut game_state);
        Self::send_top_update(clock_update.iteration_id, &mut game_state);
        Self::send_bottom_update(clock_update.iteration_id, &mut game_state);
        Self::send_top_left_update(clock_update.iteration_id, &mut game_state);
        Self::send_top_right_update(clock_update.iteration_id, &mut game_state);
        Self::send_bottom_left_update(clock_update.iteration_id, &mut game_state);
        Self::send_bottom_right_update(clock_update.iteration_id, &mut game_state);

        cast_drawable(&update);
    }

    fn handle_cast_update_left_i(_src: InstanceId, col_update: UpdateCol) {
        log::debug!("Left Update");
        let mut state = STATE.get().unwrap().lock().unwrap();

        state.handle_update(Update::Left(col_update.clone()), col_update.iteration);
    }

    fn handle_cast_update_right_i(_src: InstanceId, col_update: UpdateCol) {
        log::debug!("Right Update");
        let mut state = STATE.get().unwrap().lock().unwrap();

        state.handle_update(Update::Right(col_update.clone()), col_update.iteration);
    }

    fn handle_cast_update_bottom_i(_src: InstanceId, row_update: UpdateRow) {
        log::debug!("Bottom Update");
        let mut state = STATE.get().unwrap().lock().unwrap();

        state.handle_update(Update::Bottom(row_update.clone()), row_update.iteration);
    }

    fn handle_cast_update_top_i(_src: InstanceId, row_update: UpdateRow) {
        log::debug!("Top Update");
        let mut state = STATE.get().unwrap().lock().unwrap();

        state.handle_update(Update::Top(row_update.clone()), row_update.iteration);
    }

    fn handle_cast_update_top_left_i(_src: InstanceId, corner_update: UpdateCorner) {
        log::debug!("Top Left Update");
        let mut state = STATE.get().unwrap().lock().unwrap();

        state.handle_update(Update::TopLeft(corner_update.clone()), corner_update.iteration);
    }
    fn handle_cast_update_top_right_i(_src: InstanceId, corner_update: UpdateCorner) {
        log::debug!("Top Right Update");
        let mut state = STATE.get().unwrap().lock().unwrap();

        state.handle_update(Update::TopRight(corner_update.clone()), corner_update.iteration);
    }
    fn handle_cast_update_bottom_left_i(_src: InstanceId, corner_update: UpdateCorner) {
        log::debug!("Bottom Left Update");
        let mut state = STATE.get().unwrap().lock().unwrap();

        state.handle_update(Update::BottomLeft(corner_update.clone()), corner_update.iteration);
    }
    fn handle_cast_update_bottom_right_i(_src: InstanceId, corner_update: UpdateCorner) {
        log::debug!("Bottom Right Update");
        let mut state = STATE.get().unwrap().lock().unwrap();

        state.handle_update(Update::BottomRight(corner_update.clone()), corner_update.iteration);
    }

    fn handle_internal(data: &[u8]) {
        log::info!("Internal Message.");
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Actor Started");
        let payload_str = payload.map(|p| str::from_utf8(p).unwrap()).unwrap_or("");
        let config = configuration::Configuration::parse(payload_str);
        log::info!("Config: {config:?}");

        let mut gs = GameState::new();

        if config.draw_border {
            gs.draw_border = true;
        }

        if config.corner_blocks {
            gs.spawn_block(0, 0);
            gs.spawn_block(0, 14);
            gs.spawn_block(14, 0);
            gs.spawn_block(14, 14);
        }

        STATE.set(std::sync::Mutex::new(gs)).unwrap();
        CONFIGURATION.set(config).unwrap();
    }

    fn handle_stop() {
        log::info!("Actor Stopped");
    }
}

impl GameOfLife {
    fn send_right_update(iteration: u64, game_state: &mut GameState) {
        let mut new_state = [Field {
            state: FieldState::Dead,
            last_change_iteration: 0,
        }; 16];

        for i in 0..16 {
            new_state[i] = game_state.map[i + 1][16];
        }

        let update_msg = UpdateCol {
            iteration,
            new_state: new_state,
        };

        cast_update_right_o(&update_msg);
    }

    fn send_left_update(iteration: u64, game_state: &mut GameState) {
        let mut new_state = [Field {
            state: FieldState::Dead,
            last_change_iteration: 0,
        }; 16];

        for i in 0..16 {
            new_state[i] = game_state.map[i + 1][1];
        }

        let update_msg = UpdateCol {
            iteration,
            new_state: new_state,
        };

        cast_update_left_o(&update_msg);
    }

    fn send_top_update(iteration: u64, game_state: &mut GameState) {
        let mut new_state = [Field {
            state: FieldState::Dead,
            last_change_iteration: 0,
        }; 16];

        for i in 0..16 {
            new_state[i] = game_state.map[1][i + 1];
        }

        let update_msg = UpdateRow {
            iteration,
            new_state: new_state,
        };

        cast_update_top_o(&update_msg);
    }

    fn send_bottom_update(iteration: u64, game_state: &mut GameState) {
        let mut new_state = [Field {
            state: FieldState::Dead,
            last_change_iteration: 0,
        }; 16];

        for i in 0..16 {
            new_state[i] = game_state.map[16][i + 1];
        }

        let update_msg = UpdateRow {
            iteration,
            new_state: new_state,
        };

        cast_update_bottom_o(&update_msg);
    }

    fn send_top_left_update(iteration: u64, game_state: &mut GameState) {
        let update_msg = UpdateCorner {
            iteration,
            new_state: game_state.map[1][1],
        };

        cast_update_top_left_o(&update_msg);
    }

    fn send_top_right_update(iteration: u64, game_state: &mut GameState) {
        let update_msg = UpdateCorner {
            iteration,
            new_state: game_state.map[1][16],
        };

        cast_update_top_right_o(&update_msg);
    }

    fn send_bottom_left_update(iteration: u64, game_state: &mut GameState) {
        let update_msg = UpdateCorner {
            iteration,
            new_state: game_state.map[16][1],
        };

        cast_update_bottom_left_o(&update_msg);
    }

    fn send_bottom_right_update(iteration: u64, game_state: &mut GameState) {
        let update_msg = UpdateCorner {
            iteration,
            new_state: game_state.map[16][16],
        };

        cast_update_bottom_right_o(&update_msg);
    }
}
