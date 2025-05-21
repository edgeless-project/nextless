// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#![no_std]

#[derive(Clone)]
pub struct ActorId {
    pub node_id: [u8; 16],
    pub component_id: [u8; 16],
}

pub struct Port<'a>(pub &'a str);

pub struct Message<'a>(pub &'a [u8]);

pub enum CallRet<'a> {
    NoReply,
    Reply(allocator_api2::vec::Vec<u8, &'a dyn allocator_api2::alloc::Allocator>),
    Err,
}

pub enum LogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("Internal error on the host node.")]
    Internal,
    #[error("Actor used unknown alias.")]
    BadAlias,
    #[error("Actor exceeded resource limit.")]
    ResourceLimit,
    #[error("Actor attemted unautorized interaction.")]
    Forbidden,
}

#[derive(Debug, thiserror::Error)]
pub enum ActorError {
    #[error("Internal error in actor infrastructure.")]
    Internal,
    #[error("Error in developer-provided handler.")]
    Handler,
    #[error("Error when calling host: {0}.")]
    Host(HostError),
    #[error("Actor received message targeting undefined port.")]
    UndefinedPort,
}

pub type HostResult<T> = core::result::Result<T, HostError>;
pub type ActorResult<T> = core::result::Result<T, ActorError>;

// Guest -> Host
pub trait HostApi<'a> {
    fn cast(&mut self, output_port: Port, msg: Message) -> HostResult<()>;
    fn cast_raw(&mut self, dst_id: ActorId, dst_input_port: Port, msg: Message) -> HostResult<()>;
    fn delayed_cast(&mut self, delay_ms: u64, output_port: Port, msg: Message) -> HostResult<()>;
    fn call_raw(&'a mut self, dst_id: ActorId, dst_port: Port, msg: Message) -> HostResult<CallRet<'a>>;
    fn call(&'a mut self, output_port: Port, msg: Message) -> HostResult<CallRet<'a>>;
    fn telemetry_log(&mut self, level: LogLevel, component: &str, msg: &str) -> HostResult<()>;
    fn slf(&mut self) -> HostResult<ActorId>;
    fn state_sync(&mut self, state: &[u8]) -> HostResult<()>;
    fn allocator(&mut self) -> &dyn allocator_api2::alloc::Allocator;
}

// Host -> Guest
pub type HandleInit = fn(Option<&[u8]>, Option<&[u8]>) -> ActorResult<()>;
pub type HandleCast = fn(ActorId, &str, &[u8]) -> ActorResult<()>;
pub type HandleCall<'a> = fn(ActorId, &str, &[u8]) -> ActorResult<CallRet<'a>>;
pub type HandleStop = fn() -> ActorResult<()>;
