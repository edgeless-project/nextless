# SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
MessageGenerator = edgeless_actor_class(
    id = "message_generator",
    version = "0.1",
    outputs = [call_output("message", "String")],
    inputs = [],
    inner_structure = [source("message")],
    code = file("message_generator.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = MessageGenerator
