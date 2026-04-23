# SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
MockBroker = edgeless_actor_class(
    id = "mock_broker",
    version = "0.1",
    outputs = [
        cast_output("output{}".format(x), "test") for x in range(1, 11)
    ],
    inputs = [
        cast_input("input{}".format(x), "test") for x in range(1, 11)
    ],
    inner_structure = [
        source("output{}".format(x)) for x in range(1, 11)
    ] + [
        sink("input{}".format(x)) for x in range(1, 11)
    ],
    code = file("mock_broker.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = MockBroker
