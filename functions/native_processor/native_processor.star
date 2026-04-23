# SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
NativeProcessor = edgeless_actor_class(
    id = "native_processor",
    version = "0.1",
    outputs = [cast_output("data_out", "eft.eval.mock_sensor_value")],
    inputs = [cast_input("data_in", "eft.eval.mock_sensor_value")],
    inner_structure = [link("data_in", ["data_out"])],
    code = file("native_processor.tar.gz"),
    code_type = "RUST_NO_STD"
)

el_main = NativeProcessor
