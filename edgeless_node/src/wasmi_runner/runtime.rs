// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub struct WasmiRuntime {
    _configuration: std::collections::HashMap<String, String>,
}

impl Default for WasmiRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmiRuntime {
    pub fn new() -> Self {
        Self {
            _configuration: std::collections::HashMap::new(),
        }
    }
}
