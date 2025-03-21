// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
//
use rand::{distributions::Distribution, SeedableRng};

pub struct Random {
    rng: rand::rngs::StdRng,
}

impl Random {
    pub fn new() -> Self {
        Random {
            rng: rand::rngs::StdRng::from_entropy(),
        }
    }
}

impl super::PlacementStrategy for Random {
    fn select_candidate<'a, 'b>(
        &'a mut self,
        candidates: Vec<crate::ir::transformations::placement::Candidate<'b>>,
    ) -> Option<crate::ir::transformations::placement::Candidate<'b>> {
        if candidates.len() == 0 {
            return None;
        }
        let rv = rand::distributions::Uniform::new(0 as u64, candidates.len() as u64);
        let rnd = rv.sample(&mut self.rng);
        candidates.get(rnd as usize).and_then(|v| Some(v.clone()))
    }
}
