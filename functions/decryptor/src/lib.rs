// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use aes_gcm::{aead::Aead, KeyInit};
use edgeless_function::*;

static KEY: &[u8; 32] = &[0; 32];

struct Decryptor {}

edgeless_function::generate!(Decryptor);

impl DecryptorAPI<'_> for Decryptor {
    type EFT_EVAL_NUMBERED_TEST_MESSAGE = edgeless_function_types::eval::NumberedTestMessage;
    type EFT_EVAL_ENCRYPTED_NUMBERED_TEST_MESSAGE = edgeless_function_types::eval::EncryptedNumberedTestMessage;

    fn handle_cast_data_in(_src: InstanceId, test_msg: Self::EFT_EVAL_ENCRYPTED_NUMBERED_TEST_MESSAGE) {
        log::info!("Decryptor got message with sequence_number: {}.", test_msg.seqeunce_number);

        // Example adapted from https://docs.rs/aes-gcm/0.11.0-rc.0/aes_gcm/index.html
        let key: &aes_gcm::Key<aes_gcm::Aes256Gcm> = KEY.into();
        let cipher = aes_gcm::Aes256Gcm::new(&key);

        let mut nonce = [0u8; 12];
        nonce[..8].copy_from_slice(&test_msg.seqeunce_number.to_be_bytes());

        let nonce = aes_gcm::Nonce::<aes_gcm::aead::consts::U12>::from(nonce);
        let plaintext_bytes = cipher.decrypt(&nonce, test_msg.payload.as_ref()).expect("decrypt failed");
        let plaintext = String::from_utf8(plaintext_bytes).expect("string could not be parsed");

        let encrypted_message = Self::EFT_EVAL_NUMBERED_TEST_MESSAGE {
            seqeunce_number: test_msg.seqeunce_number,
            payload: plaintext,
        };

        cast_data_out(&encrypted_message);
    }

    fn handle_internal(_data: &[u8]) {
        log::info!("Decryptor handle_internal called");
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();

        log::info!("Decryptor Started.");
    }

    fn handle_stop() {
        log::info!("Decryptor Stopped");
    }
}
