# SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
HTTPRequestor = edgeless_actor_class(
    id = "http_requestor",
    version = "0.1",
    outputs = [call_output("http_out", "eft.http.request", "eft.http.response")],
    inputs = [],
    inner_structure = [source("http_out")],
    code = file("http_requestor.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = HTTPRequestor
