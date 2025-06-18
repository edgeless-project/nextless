// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub mod random;
pub mod round_robin;
pub mod weighted_random;

pub trait PlacementStrategy: Send + Sync {
    type GlobalState: Default + Send + Sync;
    fn select_candidate<'b>(&mut self, candidates: Vec<super::Candidate<'b>>, global_state: &Self::GlobalState) -> Option<super::Candidate<'b>>;

    fn new() -> Self;
}
