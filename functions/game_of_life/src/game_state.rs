// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

// https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life
// https://docs.rs/embedded-graphics/latest/embedded_graphics/index.html
// https://github.com/EmbersArc/rpi_led_panel/blob/main/examples/drawing.rs

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::prelude::*;
use font8x8::UnicodeFonts;

#[derive(Debug)]
pub struct GameState {
    // The last iteration for which we have run the game logic.
    pub iteration: u64,
    // Store the map with padding for the external updates.
    // The inner 16x16 block represents the instance's state.
    pub map: [[Field; 18]; 18],
    cached_updates: std::collections::BTreeMap<u64, Vec<Update>>,
    // Track which border regions we have received updates for.
    // If we don't reset those regions, this might lead to permanent alive fields in the border regions.
    peer_updates_received: ReceivedUpdates,
    pub error_occured: Option<GameError>,
    pub draw_border: bool,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum FieldState {
    Dead,
    Alive,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Field {
    pub state: FieldState,
    pub last_change_iteration: u64,
}

impl Default for FieldState {
    fn default() -> Self {
        FieldState::Dead
    }
}

#[derive(Debug)]
pub enum Update {
    Bottom(crate::messages::UpdateRow),
    Top(crate::messages::UpdateRow),
    Right(crate::messages::UpdateCol),
    Left(crate::messages::UpdateCol),
    TopLeft(crate::messages::UpdateCorner),
    TopRight(crate::messages::UpdateCorner),
    BottomLeft(crate::messages::UpdateCorner),
    BottomRight(crate::messages::UpdateCorner),
}

#[derive(Debug)]
pub enum GameError {
    LateUpdate,
    LostIteration,
    Multiple,
}

#[derive(Debug, Clone, Default)]
struct ReceivedUpdates {
    top: bool,
    bottom: bool,
    right: bool,
    left: bool,
    top_left: bool,
    top_right: bool,
    bottom_right: bool,
    bottom_left: bool,
}

impl GameState {
    pub fn new() -> Self {
        let map = [[Field {
            state: FieldState::Dead,
            last_change_iteration: 0,
        }; 18]; 18];

        Self {
            iteration: 0,
            map,
            cached_updates: Default::default(),
            peer_updates_received: Default::default(),
            error_occured: None,
            draw_border: false,
        }
    }

    // https://conwaylife.com/wiki/Glider
    pub fn spawn_glider(&mut self, center_y: usize, center_x: usize) {
        // Offset border
        let center_y = center_y + 1;
        let center_x = center_x + 1;

        self.set_alive(center_y - 1, center_x);
        self.set_alive(center_y, center_x + 1);
        self.set_alive(center_y + 1, center_x - 1);
        self.set_alive(center_y + 1, center_x);
        self.set_alive(center_y + 1, center_x + 1);
    }

    pub fn spawn_blinker(&mut self, center_y: usize, center_x: usize) {
        // Offset border
        let center_y = center_y + 1;
        let center_x = center_x + 1;

        self.set_alive(center_y, center_x - 1);
        self.set_alive(center_y, center_x);
        self.set_alive(center_y, center_x + 1);
    }

    pub fn spawn_block(&mut self, corner_y: usize, corner_x: usize) {
        // Offset border
        let corner_y = corner_y + 1;
        let corner_x = corner_x + 1;

        self.set_alive(corner_y, corner_x);
        self.set_alive(corner_y, corner_x + 1);

        self.set_alive(corner_y + 1, corner_x);
        self.set_alive(corner_y + 1, corner_x + 1);
    }

    pub fn spawn_char(&mut self, c: char, offset_y: usize, offset_x: usize) {
        // cf. unicode example at:
        // https://crates.io/crates/font8x8

        let fonts = font8x8::BASIC_FONTS;

        let Some(char) = fonts.get(c) else {
            return;
        };

        for (y_index, y) in char.iter().enumerate() {
            for x_index in 0..8 {
                match *y & 1 << x_index {
                    0 => self.set_dead(offset_y + 1 + y_index, offset_x + 1 + x_index),
                    _ => self.set_alive(offset_y + 1 + y_index, offset_x + 1 + x_index),
                }
            }
        }
    }

    pub fn draw(&self) -> edgeless_function_types::led_matrix::MatrixFrame {
        let mut frame = edgeless_function_types::led_matrix::MatrixFrame::new();

        // https://docs.rs/embedded-graphics/latest/embedded_graphics/primitives/rectangle/struct.Rectangle.html
        let block_style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
            .stroke_color(embedded_graphics::pixelcolor::Rgb888::WHITE)
            .stroke_width(1)
            .fill_color(embedded_graphics::pixelcolor::Rgb888::WHITE)
            .build();

        let color = if let Some(error) = &self.error_occured {
            match error {
                GameError::LateUpdate => embedded_graphics::pixelcolor::Rgb888::GREEN,
                GameError::LostIteration => embedded_graphics::pixelcolor::Rgb888::BLUE,
                GameError::Multiple => embedded_graphics::pixelcolor::Rgb888::RED,
            }
        } else {
            embedded_graphics::pixelcolor::Rgb888::BLACK
        };

        frame.0.clear(color);

        if self.draw_border {
            let border_style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
                .stroke_color(embedded_graphics::pixelcolor::Rgb888::RED)
                .stroke_width(1)
                .fill_color(color)
                .build();

            embedded_graphics::primitives::Rectangle::new(
                embedded_graphics::prelude::Point::new(0, 0),
                embedded_graphics::prelude::Size::new(64, 64),
            )
            .into_styled(border_style)
            .draw(&mut frame.0)
            .unwrap();
        }

        for row_id in 1..=16 {
            for col_id in 1..=16 {
                if self.map[row_id][col_id].state == FieldState::Alive {
                    embedded_graphics::primitives::Rectangle::new(
                        embedded_graphics::prelude::Point::new(((col_id - 1) * 4) as i32, ((row_id - 1) * 4) as i32),
                        embedded_graphics::prelude::Size::new(4, 4),
                    )
                    .into_styled(block_style)
                    .draw(&mut frame.0)
                    .unwrap();
                }
            }
        }

        frame
    }

    pub fn handle_update(&mut self, update: Update, iteration: u64) {
        if self.iteration == iteration {
            self.apply_update(update);
        } else if iteration > self.iteration {
            self.cached_updates.entry(iteration).or_default().push(update);
        } else {
            self.error_occured = Some(GameError::LateUpdate);
            log::info!("Outdated Update");
        }
    }

    pub fn apply_update(&mut self, update: Update) {
        match update {
            Update::Bottom(update_row) => {
                for col in 0..16 {
                    // if self.map[17][col + 1].last_change_iteration <= update_row.new_state[col].last_change_iteration {
                    self.map[17][col + 1] = update_row.new_state[col];
                    // }
                }
                self.peer_updates_received.bottom = true;
            }
            Update::Top(update_row) => {
                for col in 0..16 {
                    // if self.map[0][col + 1].last_change_iteration <= update_row.new_state[col].last_change_iteration {
                    self.map[0][col + 1] = update_row.new_state[col];
                    // }
                }
                self.peer_updates_received.top = true;
            }
            Update::Right(update_col) => {
                for row in 0..16 {
                    // if self.map[row + 1][17].last_change_iteration <= update_col.new_state[row].last_change_iteration {
                    self.map[row + 1][17] = update_col.new_state[row];
                    // }
                }
                self.peer_updates_received.right = true;
            }
            Update::Left(update_col) => {
                for row in 0..16 {
                    // if self.map[row + 1][0].last_change_iteration <= update_col.new_state[row].last_change_iteration {
                    self.map[row + 1][0] = update_col.new_state[row];
                    // }
                }
                self.peer_updates_received.left = true;
            }
            Update::TopLeft(update_corner) => {
                // if self.map[0][0].last_change_iteration <= update_corner.new_state.last_change_iteration {
                self.map[0][0] = update_corner.new_state;
                // }
                self.peer_updates_received.top_left = true;
            }
            Update::TopRight(update_corner) => {
                // if self.map[0][17].last_change_iteration <= update_corner.new_state.last_change_iteration {
                self.map[0][17] = update_corner.new_state;
                // }
                self.peer_updates_received.top_right = true;
            }
            Update::BottomLeft(update_corner) => {
                // if self.map[17][0].last_change_iteration <= update_corner.new_state.last_change_iteration {
                self.map[17][0] = update_corner.new_state;
                // }
                self.peer_updates_received.bottom_left = true;
            }
            Update::BottomRight(update_corner) => {
                // if self.map[17][17].last_change_iteration <= update_corner.new_state.last_change_iteration {
                self.map[17][17] = update_corner.new_state;
                // }
                self.peer_updates_received.bottom_right = true;
            }
        }
    }

    pub fn logic_update(&mut self, iteration_id: u64) {
        self.clear_borders_without_updates();

        let mut new_map = self.map.clone();

        for row_id in 1..17 {
            for col_id in 1..17 {
                let mut neighbor_count = 0usize;

                // Count Neighbors
                // This was likely inspired by a solution to a coding exercise which I was unable to find.
                for row_offset in -1i32..=1 {
                    for col_offset in -1i32..=1 {
                        if row_offset == 0 && col_offset == 0 {
                            continue;
                        }

                        if self.map[(row_id + row_offset) as usize][(col_id + col_offset) as usize].state == FieldState::Alive {
                            neighbor_count += 1;
                        }
                    }
                }

                // Update Map
                if self.map[row_id as usize][col_id as usize].state == FieldState::Alive {
                    if neighbor_count == 2 || neighbor_count == 3 {
                        new_map[row_id as usize][col_id as usize] = Field {
                            state: FieldState::Alive,
                            last_change_iteration: iteration_id,
                        };
                    } else {
                        new_map[row_id as usize][col_id as usize] = Field {
                            state: FieldState::Dead,
                            last_change_iteration: iteration_id,
                        };
                    }
                } else {
                    if neighbor_count == 3 {
                        new_map[row_id as usize][col_id as usize] = Field {
                            state: FieldState::Alive,
                            last_change_iteration: iteration_id,
                        };
                    }
                }
            }
        }

        self.map = new_map;

        if iteration_id != (self.iteration + 1) {
            if let Some(_) = self.error_occured {
                self.error_occured = Some(GameError::Multiple);
            } else {
                self.error_occured = Some(GameError::LostIteration);
            }
        }
        self.iteration = iteration_id;

        self.peer_updates_received = Default::default();

        if let Some(updates) = self.cached_updates.remove(&iteration_id) {
            for update in updates {
                self.apply_update(update);
            }
        }
    }

    pub fn clear_borders_without_updates(&mut self) {
        log::debug!("Received Updates:{:?}", self.peer_updates_received);

        if !self.peer_updates_received.bottom {
            for i in 1..17 {
                self.set_dead(17, i);
            }
        }

        if !self.peer_updates_received.top {
            for i in 1..17 {
                self.set_dead(0, i);
            }
        }

        if !self.peer_updates_received.right {
            for i in 1..17 {
                self.set_dead(i, 17);
            }
        }

        if !self.peer_updates_received.left {
            for i in 1..17 {
                self.set_dead(i, 0);
            }
        }

        if !self.peer_updates_received.top_left {
            self.set_dead(0, 0);
        }

        if !self.peer_updates_received.top_right {
            self.set_dead(0, 17);
        }

        if !self.peer_updates_received.bottom_left {
            self.set_dead(17, 0);
        }

        if !self.peer_updates_received.bottom_right {
            self.set_dead(17, 17);
        }
    }

    fn set_alive(&mut self, y: usize, x: usize) {
        if self.map[y][x].state == FieldState::Dead {
            self.map[y][x] = Field {
                state: FieldState::Alive,
                last_change_iteration: self.iteration,
            }
        }
    }

    fn set_dead(&mut self, y: usize, x: usize) {
        if self.map[y][x].state == FieldState::Alive {
            self.map[y][x] = Field {
                state: FieldState::Dead,
                last_change_iteration: self.iteration,
            }
        }
    }
}
