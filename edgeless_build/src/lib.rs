// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

pub mod native;
pub mod rust;
pub mod wasm;

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("Compiler Error(s):\n{msg}")]
    Compiler {
        msg: String,
        #[source]
        source: Option<anyhow::Error>,
    },
    #[error("Package Error(s):\n{msg}")]
    Package {
        msg: String,
        #[source]
        source: Option<anyhow::Error>,
    },
    #[error("Toolchain Error(s):\n{msg}")]
    Toolchain {
        msg: String,
        #[source]
        source: Option<anyhow::Error>,
    },
}
