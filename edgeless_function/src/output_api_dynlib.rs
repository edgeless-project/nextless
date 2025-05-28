// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub fn cast_raw(target: crate::InstanceId, port: &str, msg: &[u8]) {
    unsafe {
        crate::interface_dynlib::HOST_API
            .as_mut()
            .unwrap()
            .cast_raw(target, edgeless_actor_abi::Port(port), edgeless_actor_abi::Message(msg))
            .unwrap();
    }
}

pub fn cast(name: &str, msg: &[u8]) {
    unsafe {
        crate::interface_dynlib::HOST_API
            .as_mut()
            .unwrap()
            .cast(edgeless_actor_abi::Port(name), edgeless_actor_abi::Message(msg))
            .unwrap();
    }
}

pub fn delayed_cast(delay_ms: u64, name: &str, msg: &[u8]) {
    unsafe {
        crate::interface_dynlib::HOST_API
            .as_mut()
            .unwrap()
            .delayed_cast(delay_ms, edgeless_actor_abi::Port(name), edgeless_actor_abi::Message(msg))
            .unwrap();
    }
}

pub fn call_raw(target: crate::InstanceId, port: &str, msg: &[u8]) -> crate::CallRet {
    unsafe {
        let res = crate::interface_dynlib::HOST_API
            .as_mut()
            .unwrap()
            .call_raw(target, edgeless_actor_abi::Port(port), edgeless_actor_abi::Message(msg))
            .unwrap();

        match res {
            edgeless_actor_abi::CallRet::NoReply => crate::CallRet::NoReply,
            edgeless_actor_abi::CallRet::Reply(data) => crate::CallRet::Reply(crate::owned_data::OwnedByteBuff::new_from_slice(&data)),
            edgeless_actor_abi::CallRet::Err => crate::CallRet::Err,
        }
    }
}

pub fn call(name: &str, msg: &[u8]) -> crate::CallRet {
    unsafe {
        let res = crate::interface_dynlib::HOST_API
            .as_mut()
            .unwrap()
            .call(edgeless_actor_abi::Port(name), edgeless_actor_abi::Message(msg))
            .unwrap();

        match res {
            edgeless_actor_abi::CallRet::NoReply => crate::CallRet::NoReply,
            edgeless_actor_abi::CallRet::Reply(data) => crate::CallRet::Reply(crate::owned_data::OwnedByteBuff::new_from_slice(&data)),
            edgeless_actor_abi::CallRet::Err => crate::CallRet::Err,
        }
    }
}

pub fn telemetry_log(_level: usize, target: &str, msg: &str) {
    unsafe {
        crate::interface_dynlib::HOST_API
            .as_mut()
            .unwrap()
            .telemetry_log(edgeless_actor_abi::LogLevel::Info, target, msg)
            .unwrap();
    }
}

pub fn slf() -> crate::InstanceId {
    unsafe { crate::interface_dynlib::HOST_API.as_mut().unwrap().slf().unwrap() }
}

pub fn sync(state: &[u8]) {
    unsafe {
        crate::interface_dynlib::HOST_API.as_mut().unwrap().state_sync(state).unwrap();
    }
}
