// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(serde::Deserialize)]
pub(crate) struct MetadataRoot {
    pub(crate) packages: Vec<MetadataPackage>,
}

#[derive(serde::Deserialize)]
#[serde(tag = "reason")]
pub(crate) enum CargoBuildOutput {
    #[serde(alias = "compiler-message")]
    CompilerMessage { message: RustCMessage, target: MetadataTarget },
    // #[serde(alias = "build-finished")]
    // BuildFinished { success: bool },
    #[serde(other)]
    Unknown,
}

#[derive(serde::Deserialize)]
pub(crate) struct MetadataPackage {
    pub(crate) targets: Vec<MetadataTarget>,
    pub(crate) features: std::collections::HashMap<String, Vec<String>>,
}

#[derive(serde::Deserialize)]
pub(crate) struct MetadataTarget {
    pub(crate) crate_types: Vec<String>,
    pub(crate) name: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct RustCMessage {
    pub(crate) rendered: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct PackageListRoot {
    pub(crate) files: std::collections::HashMap<String, PackageListFile>,
}

#[derive(serde::Deserialize)]
pub(crate) struct PackageListFile {
    pub(crate) path: Option<String>,
}
