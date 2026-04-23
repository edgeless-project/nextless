# SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
Encryptor = edgeless_actor_class(
    id = "encryptor",
    version = "0.1",
    inputs = [cast_input("data_in", "eft.eval.numbered_test_message")],
    outputs = [cast_output("data_out", "eft.eval.encrypted_numbered_test_message")],
    inner_structure = [link("data_in", ["data_out"])],
    code = file("encryptor.tar.gz"),
    code_type = "RUST_NO_STD"
)

el_main = Encryptor
