// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub mod random;
pub mod weighted_random;

pub trait PlacementStrategy: Send + Sync {
    fn select_candidate<'a, 'b>(&'a mut self, candidates: Vec<super::Candidate<'b>>) -> Option<super::Candidate<'b>>;
}
