// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#![allow(clippy::needless_lifetimes)]

#[derive(Debug, PartialEq, Eq, allocative::Allocative, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    Cast,
    Call,
}

#[derive(Debug, PartialEq, Eq, allocative::Allocative, Clone, serde::Serialize, serde::Deserialize)]
pub enum Direction {
    Output,
    Input,
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    allocative::Allocative,
    starlark::any::ProvidesStaticType,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    starlark::values::Trace,
)]
pub struct PortSpec {
    #[serde(skip_serializing)]
    pub id: String,
    #[serde(skip_serializing)]
    pub direction: Direction,
    pub method: Method,
    pub data_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_data_type: Option<String>,
    pub optional: bool,
}

impl std::fmt::Display for PortSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("PortSpec({:?})", self))
    }
}

starlark::starlark_simple_value!(PortSpec);

#[starlark::values::starlark_value(type = "edgeless_port_class_C")]
impl<'v> starlark::values::StarlarkValue<'v> for PortSpec {}

impl<'v> starlark::values::UnpackValue<'v> for PortSpec {
    fn unpack_value(value: starlark::values::Value<'v>) -> Option<Self> {
        starlark::values::ValueLike::downcast_ref::<crate::port_class::PortSpec>(value).cloned()
    }
}

#[starlark::starlark_module]
pub fn edgeless_port_spec(builder: &mut starlark::environment::GlobalsBuilder) {
    fn cast_output<'v>(
        id: String,
        data: String,
        optional: Option<bool>,
        heap: &'v starlark::values::Heap,
    ) -> anyhow::Result<starlark::values::Value<'v>> {
        Ok(heap.alloc(PortSpec {
            id,
            method: Method::Cast,
            direction: Direction::Output,
            data_type: data,
            return_data_type: None,
            optional: optional.unwrap_or(true),
        }))
    }

    fn cast_input<'v>(
        id: String,
        data: String,
        optional: Option<bool>,
        heap: &'v starlark::values::Heap,
    ) -> anyhow::Result<starlark::values::Value<'v>> {
        Ok(heap.alloc(PortSpec {
            id,
            method: Method::Cast,
            direction: Direction::Input,
            data_type: data,
            return_data_type: None,
            optional: optional.unwrap_or(true),
        }))
    }

    fn call_output<'v>(
        id: String,
        data: String,
        return_data: Option<String>,
        optional: Option<bool>,
        heap: &'v starlark::values::Heap,
    ) -> anyhow::Result<starlark::values::Value<'v>> {
        Ok(heap.alloc(PortSpec {
            id,
            method: Method::Call,
            direction: Direction::Output,
            data_type: data,
            return_data_type: return_data,
            optional: optional.unwrap_or(true),
        }))
    }

    fn call_input<'v>(
        id: String,
        data: String,
        return_data: Option<String>,
        optional: Option<bool>,
        heap: &'v starlark::values::Heap,
    ) -> anyhow::Result<starlark::values::Value<'v>> {
        Ok(heap.alloc(PortSpec {
            id,
            method: Method::Call,
            direction: Direction::Input,
            data_type: data,
            return_data_type: return_data,
            optional: optional.unwrap_or(true),
        }))
    }
}
