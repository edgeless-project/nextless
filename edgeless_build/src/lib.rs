// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

// https://rust-lang-nursery.github.io/rust-cookbook/compression/tar.html

// We use a custom bare metal target for native (aarch64-edgeless-none-actor.json):
// https://lowenware.com/blog/aarch64-bare-metal-program-in-rust/

pub enum NativeTarget {
    AARCH64,
    AMD64,
}

pub fn rust_to_wasm(
    function_source_dir: String,
    enabled_features: Vec<String>,
    enable_default_features: bool,
    enable_all_features: bool,
) -> anyhow::Result<String> {
    let cargo_project_path = std::fs::canonicalize(std::path::PathBuf::from(function_source_dir.clone()))?;
    let cargo_manifest = cargo_project_path.join("Cargo.toml");

    let build_dir = std::env::temp_dir().join(format!("edgeless-{}", uuid::Uuid::new_v4()));

    let config = &cargo::util::context::GlobalContext::default()?;
    let mut ws = cargo::core::Workspace::new(&cargo_manifest, config)?;
    ws.set_target_dir(cargo::util::Filesystem::new(build_dir.clone()));

    let pack = ws.current()?;

    let lib_name = match pack.library() {
        Some(val) => val.name(),
        None => {
            return Err(anyhow::anyhow!("Cargo package does not contain library."));
        }
    };

    let mut build_config = cargo::core::compiler::BuildConfig::new(
        config,
        None,
        false,
        &["wasm32-unknown-unknown".to_string()],
        cargo::core::compiler::CompileMode::Build,
    )?;
    build_config.requested_profile = cargo::util::interning::InternedString::new("release");

    let feature_settings = cargo::core::resolver::CliFeatures {
        features: std::rc::Rc::new(
            enabled_features
                .iter()
                .map(|feat| cargo::core::FeatureValue::new(cargo::util::interning::InternedString::new(feat)))
                .collect(),
        ),
        all_features: enable_all_features,
        uses_default_features: enable_default_features,
    };

    let compile_options = cargo::ops::CompileOptions {
        build_config,
        cli_features: feature_settings,
        spec: cargo::ops::Packages::Packages(Vec::new()),
        filter: cargo::ops::CompileFilter::Default {
            required_features_filterable: false,
        },
        target_rustdoc_args: None,
        target_rustc_args: None,
        target_rustc_crate_types: None,
        rustdoc_document_private_items: false,
        honor_rust_version: Some(true),
    };

    cargo::ops::compile(&ws, &compile_options)?;

    let raw_result = build_dir
        .join(format!("wasm32-unknown-unknown/release/{lib_name}.wasm"))
        .to_str()
        .unwrap()
        .to_string();

    let out_file = build_dir.join("function.wasm").to_str().unwrap().to_string();

    println!(
        "{:?}",
        std::process::Command::new("wasm-opt")
            .args(["-Oz", &raw_result, "--all-features", "--enable-bulk-memory", "-o", &out_file])
            .status()?
    );

    Ok(out_file)
}

pub fn rust_to_dynlib(
    function_source_dir: String,
    enabled_features: Vec<String>,
    enable_default_features: bool,
    enable_all_features: bool,
    target: NativeTarget,
) -> anyhow::Result<String> {
    let cargo_project_path = std::fs::canonicalize(std::path::PathBuf::from(function_source_dir.clone()))?;
    let cargo_manifest = cargo_project_path.join("Cargo.toml");

    let build_dir = std::env::temp_dir().join(format!("edgeless-{}", uuid::Uuid::new_v4()));

    let mut config = cargo::util::context::GlobalContext::default()?;
    config
        .configure(
            0,
            false,
            None,
            false,
            false,
            false,
            &Some(build_dir.clone()),
            &["build-std=compiler_builtins,core,alloc".to_string()],
            &[],
        )
        .unwrap();

    let mut ws = cargo::core::Workspace::new(&cargo_manifest, &config)?;
    ws.set_target_dir(cargo::util::Filesystem::new(build_dir.clone()));

    // TODO(raphaelhetzel) Find a way to patch dependencies: https://github.com/rust-lang/cargo/issues/15564
    // let mut registry = ws.package_registry().unwrap();
    // registry.clear_lock();
    // registry
    //     .patch(
    //         &url::Url::parse("https://github.com/edgeless-project/nextless.git").unwrap(),
    //         &[
    //             (
    //                 &cargo::core::Dependency::new_override(
    //                     cargo::util::interning::InternedString::new("edgeless_function"),
    //                     cargo::core::SourceId::for_path(std::path::Path::new("/Users/raphael/code/nextless/edgeless_function")).unwrap(),
    //                 ),
    //                 None,
    //             ),
    //             (
    //                 &cargo::core::Dependency::new_override(
    //                     cargo::util::interning::InternedString::new("edgeless_function_core"),
    //                     cargo::core::SourceId::for_path(std::path::Path::new("/Users/raphael/code/nextless/edgeless_function_core")).unwrap(),
    //                 ),
    //                 None,
    //             ),
    //             (
    //                 &cargo::core::Dependency::new_override(
    //                     cargo::util::interning::InternedString::new("edgeless_actor_abi"),
    //                     cargo::core::SourceId::for_path(std::path::Path::new("/Users/raphael/code/nextless/edgeless_actor_abi")).unwrap(),
    //                 ),
    //                 None,
    //             ),
    //         ],
    //     )
    //     .unwrap();
    // registry.lock_patches();

    let pack = ws.current()?;

    let lib_name = match pack.library() {
        Some(val) => val.name(),
        None => {
            return Err(anyhow::anyhow!("Cargo package does not contain library."));
        }
    };

    let target_tripple = match target {
        NativeTarget::AARCH64 => "aarch64-edgeless-none-actor",
        NativeTarget::AMD64 => "amd64-edgeless-none-actor",
    };

    let mut build_config = cargo::core::compiler::BuildConfig::new(
        &config,
        None,
        false,
        &[format!("{}/aarch64-edgeless-none-actor.json", env!("CARGO_MANIFEST_DIR"))],
        cargo::core::compiler::CompileMode::Build,
    )?;
    build_config.requested_profile = cargo::util::interning::InternedString::new("release");

    let feature_settings = cargo::core::resolver::CliFeatures {
        features: std::rc::Rc::new(
            enabled_features
                .iter()
                .map(|feat| cargo::core::FeatureValue::new(cargo::util::interning::InternedString::new(feat)))
                .collect(),
        ),
        all_features: enable_all_features,
        uses_default_features: enable_default_features,
    };

    let compile_options = cargo::ops::CompileOptions {
        build_config,
        cli_features: feature_settings,
        spec: cargo::ops::Packages::Packages(Vec::new()),
        filter: cargo::ops::CompileFilter::Default {
            required_features_filterable: false,
        },
        target_rustdoc_args: None,
        target_rustc_args: None,
        target_rustc_crate_types: None,
        rustdoc_document_private_items: false,
        honor_rust_version: Some(true),
    };

    cargo::ops::compile(&ws, &compile_options)?;

    let raw_result = build_dir
        .join(format!("{target_tripple}/release/{lib_name}.actor"))
        .to_str()
        .unwrap()
        .to_string();

    Ok(raw_result)
}

pub fn package_rust(function_source_dir: String) -> anyhow::Result<String> {
    let cargo_project_path = std::fs::canonicalize(function_source_dir.clone())?;
    let cargo_manifest = cargo_project_path.join("Cargo.toml");

    let config = &cargo::util::context::GlobalContext::default()?;
    let ws = cargo::core::Workspace::new(&cargo_manifest, config)?;

    let pack = ws.current()?;

    let mut sources = cargo::sources::path::PathSource::new(pack.root(), pack.package_id().source_id(), ws.gctx());
    sources.load().unwrap();

    let source_files = sources.list_files(pack).unwrap();

    let build_file = std::env::temp_dir().join(format!("edgeless-tar-{}.tar.gz", uuid::Uuid::new_v4()));

    let tgz_file = std::fs::File::create(build_file.clone()).unwrap();

    let enc = flate2::write::GzEncoder::new(tgz_file, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);

    let sources: Vec<_> = source_files
        .iter()
        .map(|x| (x, x.strip_prefix(cargo_project_path.clone()).unwrap()))
        .collect();
    for (src_src, src_dest) in sources {
        tar.append_path_with_name(src_src.to_path_buf(), src_dest).unwrap();
    }

    Ok(build_file.to_str().unwrap().to_string())
}

pub fn unpack_rust_package(rust_tar: &[u8]) -> anyhow::Result<String> {
    let dec = flate2::read::GzDecoder::new(rust_tar);
    let mut archive = tar::Archive::new(dec);
    let out_dir = std::env::temp_dir().join(format!("edgeless-source-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(out_dir.clone())?;
    archive.unpack(out_dir.clone())?;
    Ok(out_dir.to_str().unwrap().to_string())
}
