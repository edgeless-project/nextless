// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::ir::transformations::placement::scoring::ScoreableRuntime;
use rand::{distributions::Distribution, SeedableRng};

pub struct WeightedRandom {
    rng: rand::rngs::StdRng,
}

impl super::PlacementStrategy for WeightedRandom {
    type GlobalState = ();

    fn select_candidate<'a, 'b>(
        &'a mut self,
        candidates: Vec<crate::ir::transformations::placement::Candidate<'b>>,
        _global_state: &mut Self::GlobalState,
    ) -> Option<crate::ir::transformations::placement::Candidate<'b>> {
        let highmark: f64 = candidates.iter().map(|c| c.runtime.capacity_score() as f64).sum();
        let rnd = if highmark > 0.0 {
            let rv = rand::distributions::Uniform::new(0.0, highmark);
            rv.sample(&mut self.rng)
        } else {
            0.0
        };

        let mut sum = 0.0_f64;
        for c in candidates {
            sum += c.runtime.capacity_score() as f64;
            if sum >= rnd {
                return Some(c.clone());
            }
        }
        None
    }

    fn new() -> Self {
        WeightedRandom {
            rng: rand::rngs::StdRng::from_entropy(),
        }
    }
}
