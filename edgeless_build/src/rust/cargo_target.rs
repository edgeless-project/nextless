// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct CargoTarget {
    pub(crate) arch: String,
    pub(crate) vendor: String,
    pub(crate) abi: String,
    pub(crate) crt_objects_fallback: String,
    pub(crate) data_layout: String,
    pub(crate) disable_redzone: bool,
    pub(crate) features: String,
    pub(crate) linker: String,
    pub(crate) linker_flavor: String,
    pub(crate) llvm_target: String,
    pub(crate) max_atomic_width: u32,
    pub(crate) metadata: CargoTargetMetadata,
    pub(crate) panic_strategy: String,
    pub(crate) pre_link_args: std::collections::HashMap<String, Vec<String>>,
    pub(crate) relocation_model: String,
    pub(crate) stack_probes: CargoTargetStackProbes,
    pub(crate) supported_sanitizers: Vec<String>,
    pub(crate) target_pointer_width: u16,
    pub(crate) dynamic_linking: bool,
    pub(crate) direct_access_external_data: bool,
    pub(crate) dll_prefix: String,
    pub(crate) dll_suffix: String,
    // https://stackoverflow.com/a/53900684
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cpu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) plt_by_default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) relro_level: Option<String>,
}

#[derive(serde::Serialize)]
pub(crate) struct CargoTargetMetadata {
    pub(crate) description: String,
    pub(crate) host_tools: bool,
    pub(crate) std: bool,
    pub(crate) tier: u32,
}

#[derive(serde::Serialize)]
pub(crate) struct CargoTargetStackProbes {
    pub(crate) kind: String,
}

pub(crate) fn aarch64_target(mut extra_features: Vec<String>) -> CargoTarget {
    let mut features = vec!["+v8a,+strict-align,+neon,+fp-armv8".to_string()];
    features.append(&mut extra_features);
    CargoTarget {
        arch: "aarch64".to_string(),
        vendor: "edgeless".to_string(),
        abi: "actor".to_string(),
        crt_objects_fallback: "false".to_string(),
        data_layout: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128-Fn32".to_string(),
        disable_redzone: true,
        features: features.join(","),
        linker: "rust-lld".to_string(),
        linker_flavor: "gnu-lld".to_string(),
        llvm_target: "aarch64-unknown-none".to_string(),
        max_atomic_width: 128,
        metadata: CargoTargetMetadata {
            description: "Generated aarch64 actor config (derived from aarch64-unknown-none)".to_string(),
            host_tools: false,
            std: false,
            tier: 3,
        },
        panic_strategy: "abort".to_string(),
        pre_link_args: std::collections::HashMap::from([(
            "gnu-lld".to_string(),
            vec![
                "--fix-cortex-a53-843419".to_string(),
                "--strip-all".to_string(),
                "-znorelro".to_string(),
                "-einit".to_string(),
            ],
        )]),
        relocation_model: "pic".to_string(),
        stack_probes: CargoTargetStackProbes { kind: "inline".to_string() },
        supported_sanitizers: vec!["kcfi".to_string(), "kernel-address".to_string()],
        target_pointer_width: 64,
        dynamic_linking: true,
        direct_access_external_data: true,
        dll_prefix: "".to_string(),
        dll_suffix: ".actor".to_string(),
        cpu: None,
        plt_by_default: None,
        relro_level: None,
    }
}

pub(crate) fn amd64_target(mut extra_features: Vec<String>) -> CargoTarget {
    let mut features = vec!["".to_string()];
    features.append(&mut extra_features);
    CargoTarget {
        arch: "x86_64".to_string(),
        vendor: "edgeless".to_string(),
        abi: "actor".to_string(),
        crt_objects_fallback: "false".to_string(),
        data_layout: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128".to_string(),
        disable_redzone: true,
        features: features.join(","),
        linker: "rust-lld".to_string(),
        linker_flavor: "gnu-lld".to_string(),
        llvm_target: "x86_64-unknown-none-elf".to_string(),
        max_atomic_width: 64,
        metadata: CargoTargetMetadata {
            description: "Generated amd64 actor config (derived from x86_64-unknown-none)".to_string(),
            host_tools: false,
            std: false,
            tier: 3,
        },
        panic_strategy: "abort".to_string(),
        pre_link_args: std::collections::HashMap::from([(
            "gnu-lld".to_string(),
            vec!["--strip-all".to_string(), "-znorelro".to_string(), "-einit".to_string()],
        )]),
        relocation_model: "pic".to_string(),
        stack_probes: CargoTargetStackProbes { kind: "inline".to_string() },
        supported_sanitizers: vec!["kcfi".to_string(), "kernel-address".to_string()],
        target_pointer_width: 64,
        dynamic_linking: true,
        direct_access_external_data: true,
        dll_prefix: "".to_string(),
        dll_suffix: ".actor".to_string(),
        cpu: Some("x86-64".to_string()),
        plt_by_default: Some(false),
        relro_level: Some("off".to_string()),
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_aarch64() {
        println!("{}", serde_json::to_string(&super::aarch64_target(vec![])).unwrap());
    }
}
