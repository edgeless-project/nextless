# SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
UnbalancedSource = edgeless_actor_class(
    id = "unbalanced_source",
    version = "0.1",
    outputs = [
        cast_output("frequent", "eft.eval.numbered_test_message"),
        cast_output("infrequent", "eft.eval.numbered_test_message")
    ],
    inputs = [],
    inner_structure = [
       source("frequent"),
       source("infrequent")
    ],
    code = file("unbalanced_source.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = UnbalancedSource
