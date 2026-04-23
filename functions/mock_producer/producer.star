# SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
MockProducer = edgeless_actor_class(
    id = "mock_producer",
    version = "0.1",
    outputs = [
        cast_output("output{}".format(x), "test") for x in range(1, 11)
    ],
    inputs = [],
    inner_structure = [
        source("output{}".format(x)) for x in range(1, 11)
    ],
    code = file("mock_producer.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = MockProducer
