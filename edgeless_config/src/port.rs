// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#![allow(clippy::needless_lifetimes)]

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
pub enum Mapping {
    Unmapped,
    Direct(DirectTarget),
    Any(Vec<DirectTarget>),
    All(Vec<DirectTarget>),
    Topic(String),
}

starlark::starlark_simple_value!(Mapping);

#[starlark::values::starlark_value(type = "edgeless_port_mapping_c")]
impl<'v> starlark::values::StarlarkValue<'v> for Mapping {}

impl std::fmt::Display for Mapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("PortMapping({:?})", self))
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, allocative::Allocative, Clone, starlark::values::Trace)]
pub struct DirectTarget {
    pub target_component: String,
    pub port: String,
}

#[derive(Debug, PartialEq, Eq, allocative::Allocative, starlark::any::ProvidesStaticType, serde::Serialize, serde::Deserialize, Clone)]
pub struct PortGen<MappingType> {
    pub component_id: String,
    pub port_id: String,
    pub mapping: MappingType,
    pub klass: crate::port_class::PortSpec,
}

pub type Port = PortGen<std::rc::Rc<std::cell::RefCell<Mapping>>>;
pub type FrozenPort = PortGen<Mapping>;

#[allow(clippy::needless_lifetimes)]
#[starlark::values::starlark_value(type = "edgeless_frozen_port_C")]
impl<'v> starlark::values::StarlarkValue<'v> for FrozenPort {
    type Canonical = FrozenPort;
}

#[allow(clippy::needless_lifetimes)]
#[starlark::values::starlark_value(type = "edgeless_port_C")]
impl<'v> starlark::values::StarlarkValue<'v> for Port {
    type Canonical = Port;

    fn right_shift(
        &self,
        other: starlark::values::Value<'v>,
        _heap: &'v starlark::values::Heap,
    ) -> Result<starlark::values::Value<'v>, starlark::Error> {
        if let Some(port) = starlark::values::ValueLike::downcast_ref::<Port>(other) {
            *self.mapping.borrow_mut() = Mapping::Direct(DirectTarget {
                target_component: port.component_id.clone(),
                port: port.port_id.clone(),
            });

            return Ok(other);
        };

        if let Some(ports) = <starlark::values::list::ListOf<Port> as starlark::values::UnpackValue>::unpack_value(other) {
            *self.mapping.borrow_mut() = Mapping::All(
                ports
                    .to_vec()
                    .into_iter()
                    .map(|port| DirectTarget {
                        target_component: port.component_id.clone(),
                        port: port.port_id.clone(),
                    })
                    .collect(),
            );

            return Ok(other);
        }

        if let Some(filter) = starlark::values::ValueLike::downcast_ref::<Mapping>(other) {
            *self.mapping.borrow_mut() = filter.clone();

            return Ok(other);
        }

        Ok(other)
    }

    fn left_shift(
        &self,
        other: starlark::values::Value<'v>,
        _heap: &'v starlark::values::Heap,
    ) -> Result<starlark::values::Value<'v>, starlark::Error> {
        if let Some(filter) = starlark::values::ValueLike::downcast_ref::<Mapping>(other) {
            *self.mapping.borrow_mut() = filter.clone();
        }

        Ok(other)
    }
}

impl<'v> starlark::values::AllocValue<'v> for Port {
    fn alloc_value(self, heap: &'v starlark::values::Heap) -> starlark::values::Value<'v> {
        heap.alloc_complex(self)
    }
}

impl<V> std::fmt::Display for PortGen<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Port(component={}, port_id={})", self.component_id, self.port_id))
    }
}

impl<'v> starlark::values::UnpackValue<'v> for Port {
    fn unpack_value(value: starlark::values::Value<'v>) -> Option<Self> {
        starlark::values::ValueLike::downcast_ref::<Port>(value).cloned()
    }
}

impl starlark::values::Freeze for Port {
    type Frozen = FrozenPort;

    fn freeze(self, _freezer: &starlark::values::Freezer) -> anyhow::Result<Self::Frozen> {
        Ok(FrozenPort {
            component_id: self.component_id.clone(),
            port_id: self.port_id.clone(),
            mapping: self.mapping.borrow_mut().clone(),
            klass: self.klass.clone(),
        })
    }
}

unsafe impl<'v> starlark::values::Trace<'v> for Port {
    fn trace(&mut self, tracer: &starlark::values::Tracer<'v>) {
        self.component_id.trace(tracer);
        self.port_id.trace(tracer);
        self.mapping.borrow_mut().trace(tracer);
        self.klass.trace(tracer);
    }
}

#[starlark::starlark_module]
pub fn edgeless_port(builder: &mut starlark::environment::GlobalsBuilder) {
    fn topic<'v>(topic: String, heap: &'v starlark::values::Heap) -> anyhow::Result<starlark::values::Value<'v>> {
        Ok(heap.alloc(Mapping::Topic(topic)))
    }

    fn any<'v>(items: starlark::values::Value<'v>, heap: &'v starlark::values::Heap) -> anyhow::Result<starlark::values::Value<'v>> {
        if let Some(ports) = <starlark::values::list::ListOf<Port> as starlark::values::UnpackValue>::unpack_value(items) {
            return Ok(heap.alloc(Mapping::Any(
                ports
                    .to_vec()
                    .into_iter()
                    .map(|port| DirectTarget {
                        target_component: port.component_id.clone(),
                        port: port.port_id.clone(),
                    })
                    .collect(),
            )));
        }
        Err(anyhow::anyhow!("Not a list of ports"))
    }
}
