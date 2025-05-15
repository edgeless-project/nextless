// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#![no_std]

pub struct ActorId {
    pub node_id: [u8; 16],
    pub component_id: [u8; 16],
}

pub struct Port<'a>(pub &'a str);

pub struct Message<'a>(pub &'a [u8]);

pub enum CallRet {
    NoReply,
    Reply([u8; 1500]),
    Err,
}

pub enum LogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

#[derive(Debug)]
pub enum HostError {
    Internal,
    BadAlias,
    ResourceLimit,
    Forbidden,
}

#[derive(Debug)]
pub enum ActorError {
    Internal,
}

pub type HostResult<T> = core::result::Result<T, HostError>;
pub type ActorResult<T> = core::result::Result<T, ActorError>;

// Guest -> Host
// pub type Cast = fn(output_port: Port, msg: Message) -> HostResult<()>;
// pub type CastRaw = fn(dst_id: ActorId, dst_input_port: Port, msg: Message) -> HostResult<()>;
// pub type DelayedCast = fn(delay_ms: u64, output_port: Port, msg: Message) -> HostResult<()>;
// pub type CallRaw = fn(dst_id: ActorId, dst_port: Port, msg: Message) -> HostResult<CallRet>;
// pub type Call = fn(output_port: Port, msg: Message) -> HostResult<CallRet>;
// pub type TelemetryLog = fn(level: LogLevel, &'static str, msg: &str) -> HostResult<()>;
// pub type Slf = fn() -> ActorId;
// pub type StateSync = fn(state: &[u8]);

// pub struct HostApi {
//     pub cast: Cast,
//     pub cast_raw: CastRaw,
//     pub delayed_cast: DelayedCast,
//     pub call_raw: CallRaw,
//     pub call: Call,
//     pub telemetry_log: TelemetryLog,
//     pub slf: Slf,
//     pub state_sync: StateSync,
// }

pub trait HostApi {
    fn cast(&mut self, output_port: Port, msg: Message) -> HostResult<()>;
    fn cast_raw(&mut self, dst_id: ActorId, dst_input_port: Port, msg: Message) -> HostResult<()>;
    fn delayed_cast(&mut self, delay_ms: u64, output_port: Port, msg: Message) -> HostResult<()>;
    fn call_raw(&mut self, dst_id: ActorId, dst_port: Port, msg: Message) -> HostResult<CallRet>;
    fn call(&mut self, output_port: Port, msg: Message) -> HostResult<CallRet>;
    fn telemetry_log(&mut self, level: LogLevel, component: &str, msg: &str) -> HostResult<()>;
    fn slf(&mut self) -> HostResult<ActorId>;
    fn state_sync(&mut self, state: &[u8]) -> HostResult<()>;
    fn alloc(&mut self, layout: core::alloc::Layout) -> HostResult<*mut u8>;
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: core::alloc::Layout) -> HostResult<()>;
}
