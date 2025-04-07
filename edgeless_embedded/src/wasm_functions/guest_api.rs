// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

// TODO(raphaelhetzel) Move this to a generic module once we have more than one embedded runtime

/// Each function instance can import a set of functions that need to be implemented on the host-side.
/// This provides the generic host-side implementation of these functions.
/// Those need to be made available to the guest using a virtualization-specific interface/binding.
pub struct GuestAPIHost {
    pub instance_id: edgeless_api_core::instance_id::InstanceId,
    pub data_plane: core::cell::RefCell<crate::dataplane::EmbeddedDataplaneHandle>,
}

/// Errors to be reported by the host side of the guest binding.
/// This may need to be bridged into the runtime by the virtualization-specific runtime implementation.
#[derive(Debug)]
pub enum GuestAPIError {
    UnknownAlias,
    Unimplemented,
    Internal,
    BadParameter,
}

impl From<crate::dataplane::DataplaneError> for GuestAPIError {
    fn from(dataplane_error: crate::dataplane::DataplaneError) -> Self {
        match dataplane_error {
            crate::dataplane::DataplaneError::BadParameter => Self::BadParameter,
            crate::dataplane::DataplaneError::Internal => Self::Internal,
            crate::dataplane::DataplaneError::UnknownAlias => Self::UnknownAlias,
        }
    }
}

impl GuestAPIHost {
    pub async fn cast_alias(&self, alias: &str, msg: &[u8]) -> Result<(), GuestAPIError> {
        self.data_plane.borrow_mut().send_alias(alias, msg).await.map_err(GuestAPIError::from)?;
        Ok(())
    }

    pub async fn cast_raw(
        &self,
        target: edgeless_api_core::instance_id::InstanceId,
        target_port: edgeless_api_core::port::Port<32>,
        msg: &[u8],
    ) -> Result<(), GuestAPIError> {
        self.data_plane
            .borrow_mut()
            .send(self.instance_id, target, target_port, msg)
            .await
            .map_err(GuestAPIError::from)?;
        Ok(())
    }

    pub async fn call_alias(&self, _alias: &str, _msg: &[u8]) -> Result<crate::dataplane::CallRet, GuestAPIError> {
        Err(GuestAPIError::Unimplemented)
    }

    pub async fn call_raw(
        &self,
        _target: edgeless_api_core::instance_id::InstanceId,
        _target_port: edgeless_api_core::port::Port<32>,
        _msg: &[u8],
    ) -> Result<crate::dataplane::CallRet, GuestAPIError> {
        Err(GuestAPIError::Unimplemented)
    }

    pub async fn telemetry_log(&self, lvl: super::TelemetryLogLevel, target: &str, msg: &str) {
        let lvl = match lvl {
            crate::wasm_functions::TelemetryLogLevel::Error => log::Level::Error,
            crate::wasm_functions::TelemetryLogLevel::Warn => log::Level::Warn,
            crate::wasm_functions::TelemetryLogLevel::Info => log::Level::Info,
            crate::wasm_functions::TelemetryLogLevel::Debug => log::Level::Debug,
            crate::wasm_functions::TelemetryLogLevel::Trace => log::Level::Trace,
        };
        log::log!(lvl, "Function Log {}-{}: {}", self.instance_id, target, msg);
    }

    pub async fn slf(&self) -> edgeless_api_core::instance_id::InstanceId {
        self.instance_id
    }

    pub async fn delayed_cast(&self, _delay: u64, _target_alias: &str, _payload: &[u8]) -> Result<(), GuestAPIError> {
        Err(GuestAPIError::Unimplemented)
    }

    pub async fn sync(&self, _serialized_state: &[u8]) -> Result<(), GuestAPIError> {
        Err(GuestAPIError::Unimplemented)
    }
}
