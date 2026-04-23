# SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
UnusedInput = edgeless_actor_class(
    id = "unused_input",
    version = "0.1",
    outputs = [cast_output("data_out", "eft.eval.numbered_test_message")],
    inputs = [cast_input("data_in", "eft.eval.numbered_test_message"), cast_input("unused_in", "eft.eval.numbered_test_message")],
    inner_structure = [link("data_in", ["data_out"]), link("unused_in", ["data_out"])],
    code = file("unused_input.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = UnusedInput
