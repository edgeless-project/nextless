// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub trait ScoreableRuntime {
    fn load_score(&self) -> f32;
    fn efficiency_score(&self) -> f32;
    fn capacity_score(&self) -> f32;
}

impl ScoreableRuntime for crate::ir::Runtime<'_> {
    fn load_score(&self) -> f32 {
        match self {
            crate::ir::Runtime::WasmBase(wasm_runtime) => {
                let cpu_load_score = 1.0 - (wasm_runtime.runtime_info().cpu_load() / wasm_runtime.num_cores() as f32);
                let memory_load_score = 1.0 - (wasm_runtime.runtime_info().mem_used() as f32 / wasm_runtime.mem_size_bytes() as f32);
                0.5 * cpu_load_score + 0.5 * memory_load_score
            }
            crate::ir::Runtime::Native(native_runtime) => {
                let cpu_load_score = 1.0 - (native_runtime.runtime_info().cpu_load() / native_runtime.num_cores() as f32);
                let memory_load_score = 1.0 - (native_runtime.runtime_info().mem_used() as f32 / native_runtime.mem_size_bytes() as f32);
                0.5 * cpu_load_score + 0.5 * memory_load_score
            }
        }
    }

    fn efficiency_score(&self) -> f32 {
        match self {
            super::Runtime::WasmBase(_wasm_runtime) => 0.95,
            super::Runtime::Native(_native_runtime) => 1.0,
        }
    }

    fn capacity_score(&self) -> f32 {
        match self {
            super::Runtime::WasmBase(wasm_runtime) => (wasm_runtime.cpu_freq_hz() * wasm_runtime.num_cores() as f32) / (10_000_000_000.0 * 512.0),
            super::Runtime::Native(native_runtime) => (native_runtime.cpu_freq_hz() * native_runtime.num_cores() as f32) / (10_000_000_000.0 * 512.0),
        }
    }
}
