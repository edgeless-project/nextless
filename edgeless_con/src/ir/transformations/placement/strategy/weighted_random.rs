// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::ir::transformations::placement::scoring::ScoreableRuntime;
use rand::{distributions::Distribution, seq::SliceRandom, SeedableRng};

pub struct WeightedRandom {
    rng: rand::rngs::StdRng,
}

impl super::PlacementStrategy for WeightedRandom {
    type GlobalState = ();

    fn select_candidate<'b>(
        &mut self,
        candidates: Vec<crate::ir::transformations::placement::Candidate<'b>>,
        _global_state: &Self::GlobalState,
    ) -> Option<crate::ir::transformations::placement::Candidate<'b>> {
        // We just assume all candidates are of the same type.
        if let Some(c) = candidates.get(0) {
            match c {
                crate::ir::transformations::placement::Candidate::Actor(_) => self.select_actor_candidate(candidates),
                crate::ir::transformations::placement::Candidate::Resource(_) => self.select_resource_candidate(candidates),
            }
        } else {
            None
        }
    }

    fn new() -> Self {
        WeightedRandom {
            rng: rand::rngs::StdRng::from_entropy(),
        }
    }
}
impl WeightedRandom {
    fn select_actor_candidate<'b>(
        &mut self,
        candidates: Vec<crate::ir::transformations::placement::Candidate<'b>>,
    ) -> Option<crate::ir::transformations::placement::Candidate<'b>> {
        let actor_candidates: Vec<_> = candidates
            .iter()
            .filter_map(|c| match c {
                crate::ir::transformations::placement::Candidate::Actor(actor_candidate) => Some(actor_candidate),
                crate::ir::transformations::placement::Candidate::Resource(_) => {
                    tracing::warn!("Bad candidate for actor selection.");
                    None
                }
            })
            .collect();

        let highmark: f64 = actor_candidates.iter().map(|c| c.runtime.capacity_score()).sum();
        let rnd = if highmark > 0.0 {
            let rv = rand::distributions::Uniform::new(0.0, highmark);
            rv.sample(&mut self.rng)
        } else {
            0.0
        };

        let mut sum = 0.0_f64;
        for c in actor_candidates {
            sum += c.runtime.capacity_score();
            if sum >= rnd {
                return Some(crate::ir::transformations::placement::Candidate::Actor(c.clone()));
            }
        }
        None
    }

    fn select_resource_candidate<'b>(
        &mut self,
        candidates: Vec<crate::ir::transformations::placement::Candidate<'b>>,
    ) -> Option<crate::ir::transformations::placement::Candidate<'b>> {
        // Just random selection
        // https://stackoverflow.com/a/34215930
        candidates.choose(&mut self.rng).cloned()
    }
}
