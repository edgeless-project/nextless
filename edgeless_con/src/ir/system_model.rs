// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub trait Node {
    fn node_id(&self) -> edgeless_api::function_instance::NodeId;
    fn cluster_id(&self) -> edgeless_api::function_instance::NodeId;
    fn available_runtimes<'a>(&'a self) -> Runtimes<'a>;
    fn available_resource_providers<'a>(&'a self) -> ResourceProviders<'a>;
    fn available_link_types(&self) -> LinkProviders;
    fn labels(&self) -> Vec<String>;
    fn is_proxy(&self) -> bool;
}

pub type LinkProviders = std::collections::HashMap<edgeless_api::link::LinkType, edgeless_api::link::LinkProviderId>;
pub type Nodes<'a> = std::collections::HashMap<edgeless_api::function_instance::NodeId, &'a dyn Node>;

#[derive(Clone)]
pub enum Runtime<'a> {
    WasmBase(
        &'a dyn WasmRuntime,
        std::collections::BTreeSet<crate::ir::behavior::dialect::wasm::WasmDialectFeatures>,
    ),
    NativeBase(
        &'a dyn NativeRuntime,
        std::collections::BTreeSet<crate::ir::behavior::dialect::native_dyanamic::NativeDynamicDialectFeatures>,
    ),
}

impl Runtime<'_> {
    pub fn supported_dialect(&self) -> super::behavior::dialect::DialectType {
        match self {
            Runtime::WasmBase(_, features) => super::behavior::dialect::DialectType {
                base_type: super::behavior::dialect::wasm::ID,
                features: features
                    .iter()
                    .map(|f| crate::ir::behavior::dialect::DialectFeature::Wasm(f.clone()))
                    .collect(),
            },
            Runtime::NativeBase(_, features) => super::behavior::dialect::DialectType {
                base_type: super::behavior::dialect::native_dyanamic::ID,
                features: features
                    .iter()
                    .map(|f| crate::ir::behavior::dialect::DialectFeature::Native(f.clone()))
                    .collect(),
            },
        }
    }
}

pub type Runtimes<'a> = std::collections::HashMap<String, Runtime<'a>>;

pub trait WasmRuntime {
    fn num_cores(&self) -> u32;
    fn cpu_freq_hz(&self) -> f32;
    #[allow(unused)]
    fn mem_size_bytes(&self) -> u32;
    #[allow(unused)]
    fn runtime_info(&self) -> Option<Box<dyn WasmRuntimeInfo>>;
}

pub trait NativeRuntime {
    fn num_cores(&self) -> u32;
    fn cpu_freq_hz(&self) -> f32;
    #[allow(unused)]
    fn mem_size_bytes(&self) -> u32;
    #[allow(unused)]
    fn runtime_info(&self) -> Option<Box<dyn WasmRuntimeInfo>>;
}

pub trait WasmRuntimeInfo {
    fn cpu_load(&self) -> f32;
    fn mem_used(&self) -> f32;
    fn running_instances(&self) -> u32;
}

pub trait ResourceProvider {
    fn class_type(&self) -> String;
    // TODO(raphael) Update to use Ports.
    #[allow(unused)]
    fn outputs(&self) -> Vec<String>;
}

pub type ResourceProviders<'a> = std::collections::HashMap<String, &'a dyn ResourceProvider>;

// Read-only view of a peer-cluster's state
pub trait Cluster {}

pub type Clusters<'a> = std::collections::HashMap<edgeless_api::function_instance::NodeId, &'a dyn Cluster>;
