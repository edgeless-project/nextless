// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

#[derive(Debug, PartialEq, Eq)]
pub struct NumberedTestMessage {
    pub sequence_number: u64,
    pub payload: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EncryptedNumberedTestMessage {
    pub sequence_number: u64,
    pub payload: Vec<u8>,
}

#[derive(Clone)]
pub struct MockSensorValue {
    pub sequence_number: u64,
    pub sensor_id: SensorId,
    pub value: f64,
}

#[derive(Clone)]
pub struct SensorId {
    pub node_id: [u8; 16],
    pub component_id: [u8; 16],
}

impl<'a> edgeless_function_core::Serialize<'a> for NumberedTestMessage {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        let mut out = Vec::with_capacity(self.payload.len() + 8);
        out.extend_from_slice(&self.sequence_number.to_be_bytes());
        out.extend_from_slice(&(self.payload.len() as u64).to_be_bytes());
        out.extend_from_slice(self.payload.as_bytes());
        out
    }
}

impl<'a> edgeless_function_core::Deserialize<'a> for NumberedTestMessage {
    fn deserialize(raw: &'a [u8]) -> Self {
        assert!(raw.len() >= 16);
        let payload_len = u64::from_be_bytes(raw[8..16].try_into().unwrap());

        let mut payload = Vec::with_capacity(payload_len as usize);
        if payload_len > 0 {
            payload.extend_from_slice(&raw[16..]);
        }

        let payload = String::from_utf8(payload).unwrap();
        Self {
            sequence_number: u64::from_be_bytes(raw[0..8].try_into().unwrap()),
            payload,
        }
    }
}

impl<'a> edgeless_function_core::Serialize<'a> for EncryptedNumberedTestMessage {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        let mut out = Vec::with_capacity(self.payload.len() + 8);
        out.extend_from_slice(&self.sequence_number.to_be_bytes());
        out.extend_from_slice(&(self.payload.len() as u64).to_be_bytes());
        out.extend_from_slice(&self.payload);
        out
    }
}

impl<'a> edgeless_function_core::Deserialize<'a> for EncryptedNumberedTestMessage {
    fn deserialize(raw: &'a [u8]) -> Self {
        assert!(raw.len() >= 16);
        let payload_len = u64::from_be_bytes(raw[8..16].try_into().unwrap());
        let mut payload = Vec::with_capacity(payload_len as usize);
        if payload_len > 0 {
            payload.extend_from_slice(&raw[16..]);
        }
        Self {
            sequence_number: u64::from_be_bytes(raw[0..8].try_into().unwrap()),
            payload,
        }
    }
}

impl<'a> edgeless_function_core::Serialize<'a> for MockSensorValue {
    fn serialize(&'a self) -> impl core::convert::AsRef<[u8]> {
        let mut out = Vec::with_capacity(size_of::<MockSensorValue>());
        out.extend_from_slice(&self.sequence_number.to_be_bytes());
        out.extend_from_slice(&self.value.to_be_bytes());
        out.extend_from_slice(&self.sensor_id.node_id);
        out.extend_from_slice(&self.sensor_id.component_id);
        out
    }
}

impl<'a> edgeless_function_core::Deserialize<'a> for MockSensorValue {
    fn deserialize(raw: &'a [u8]) -> Self {
        assert!(raw.len() >= size_of::<MockSensorValue>());

        let sequence_number = u64::from_be_bytes(raw[0..8].try_into().unwrap());
        let value = f64::from_be_bytes(raw[8..16].try_into().unwrap());
        let node_id: [u8; 16] = raw[16..32].try_into().unwrap();
        let component_id: [u8; 16] = raw[32..48].try_into().unwrap();

        Self {
            sequence_number,
            sensor_id: SensorId { node_id, component_id },
            value,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use edgeless_function_core::{Deserialize, Serialize};

    #[test]
    fn numbered_test_message() {
        let m = NumberedTestMessage {
            sequence_number: 1,
            payload: "Test".to_string(),
        };

        let s = m.serialize();
        let d = NumberedTestMessage::deserialize(s.as_ref());
        assert_eq!(d, m);
    }

    #[test]
    fn emty_numbered_test_message() {
        let m = NumberedTestMessage {
            sequence_number: 1,
            payload: "".to_string(),
        };

        let s = m.serialize();
        let d = NumberedTestMessage::deserialize(s.as_ref());
        assert_eq!(d, m);
    }

    #[test]
    fn encryted_numbered_test_message() {
        let m = EncryptedNumberedTestMessage {
            sequence_number: 1,
            payload: Vec::from([0; 16]),
        };

        let s = m.serialize();
        let d = EncryptedNumberedTestMessage::deserialize(s.as_ref());
        assert_eq!(d, m);
    }

    #[test]
    fn empty_encrypted_numbered_test_message() {
        let m = EncryptedNumberedTestMessage {
            sequence_number: 1,
            payload: Vec::new(),
        };

        let s = m.serialize();
        let d = EncryptedNumberedTestMessage::deserialize(s.as_ref());
        assert_eq!(d, m);
    }
}
