// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use core::convert::TryInto;

#[derive(Debug)]
pub struct Iteration {
    pub iteration_id: u64,
}

#[derive(Debug, Clone)]
pub struct UpdateRow {
    pub iteration: u64,
    pub new_state: [super::Field; 16],
}

#[derive(Debug, Clone)]
pub struct UpdateCol {
    pub iteration: u64,
    pub new_state: [super::Field; 16],
}

#[derive(Debug, Clone)]
pub struct UpdateCorner {
    pub iteration: u64,
    pub new_state: super::Field,
}

impl<'a> edgeless_function::Serialize<'a> for UpdateRow {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        let mut out = [0u8; (9 * 16 + 8)];

        for i in 0..16 {
            if self.new_state[i].state == super::FieldState::Alive {
                out[i * 9] = 1;
            }
            out[(i * 9 + 1)..(i * 9 + 9)].copy_from_slice(&self.new_state[i].last_change_iteration.to_le_bytes());
        }

        out[16 * 9..16 * 9 + 8].copy_from_slice(&self.iteration.to_le_bytes());

        out
    }
}

impl<'a> edgeless_function::Deserialize<'a> for UpdateRow {
    fn deserialize(raw: &'a [u8]) -> Self {
        let mut updates = [super::Field {
            state: super::FieldState::Dead,
            last_change_iteration: 0,
        }; 16];

        for i in 0..16 {
            let new_state = if raw[i * 9] >= 1 {
                super::FieldState::Alive
            } else {
                super::FieldState::Dead
            };
            let last_change_iteration = u64::from_le_bytes(raw[(i * 9 + 1)..(i * 9 + 9)].try_into().unwrap());

            updates[i] = super::Field {
                state: new_state,
                last_change_iteration,
            }
        }

        let iteration = u64::from_le_bytes(raw[(16 * 9)..(16 * 9 + 8)].try_into().unwrap());

        UpdateRow {
            new_state: updates,
            iteration,
        }
    }
}

impl<'a> edgeless_function::Serialize<'a> for UpdateCol {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        let mut out = [0u8; (9 * 16 + 8)];

        for i in 0..16 {
            if self.new_state[i].state == super::FieldState::Alive {
                out[i * 9] = 1;
            }
            out[(i * 9 + 1)..(i * 9 + 9)].copy_from_slice(&self.new_state[i].last_change_iteration.to_le_bytes());
        }

        out[16 * 9..16 * 9 + 8].copy_from_slice(&self.iteration.to_le_bytes());

        out
    }
}

impl<'a> edgeless_function::Deserialize<'a> for UpdateCol {
    fn deserialize(raw: &'a [u8]) -> Self {
        let mut updates = [super::Field {
            state: super::FieldState::Dead,
            last_change_iteration: 0,
        }; 16];

        for i in 0..16 {
            let new_state = if raw[i * 9] >= 1 {
                super::FieldState::Alive
            } else {
                super::FieldState::Dead
            };
            let iteration = u64::from_le_bytes(raw[(i * 9 + 1)..(i * 9 + 9)].try_into().unwrap());

            updates[i] = super::Field {
                state: new_state,
                last_change_iteration: iteration,
            }
        }

        let iteration = u64::from_le_bytes(raw[(16 * 9)..(16 * 9 + 8)].try_into().unwrap());

        UpdateCol {
            new_state: updates,
            iteration,
        }
    }
}

impl<'a> edgeless_function::Serialize<'a> for UpdateCorner {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        let mut out = [0u8; 17];

        out[..8].copy_from_slice(&self.iteration.to_le_bytes());
        out[8..16].copy_from_slice(&self.new_state.last_change_iteration.to_le_bytes());

        if self.new_state.state == super::FieldState::Alive {
            out[16] = 1;
        }

        out
    }
}

impl<'a> edgeless_function::Deserialize<'a> for UpdateCorner {
    fn deserialize(raw: &'a [u8]) -> Self {
        let iteration = u64::from_le_bytes(raw[..8].try_into().unwrap());
        let last_change = u64::from_le_bytes(raw[8..16].try_into().unwrap());

        let new_state = if raw[16] >= 1 {
            super::FieldState::Alive
        } else {
            super::FieldState::Dead
        };

        UpdateCorner {
            new_state: super::Field {
                last_change_iteration: last_change,
                state: new_state,
            },
            iteration,
        }
    }
}

impl<'a> edgeless_function::Serialize<'a> for Iteration {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        self.iteration_id.to_le_bytes()
    }
}

impl<'a> edgeless_function::Deserialize<'a> for Iteration {
    fn deserialize(raw: &'a [u8]) -> Self {
        let iteration_id = u64::from_le_bytes(raw.try_into().unwrap());

        Self { iteration_id }
    }
}
