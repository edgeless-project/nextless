// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub mod random;
pub mod round_robin;
pub mod weighted_random;

pub trait PlacementStrategy: Send + Sync {
    type GlobalState: Default;
    fn select_candidate<'a, 'b>(
        &'a mut self,
        candidates: Vec<super::Candidate<'b>>,
        global_state: &mut Self::GlobalState,
    ) -> Option<super::Candidate<'b>>;

    fn new() -> Self;
}
