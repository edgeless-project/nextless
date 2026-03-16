// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

// Copied from ../functions/game_of_life/src/messages.rs.
// This should be moved to a crate.

#[derive(Debug)]
pub struct Iteration {
    pub iteration_id: u64,
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
