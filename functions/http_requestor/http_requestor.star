HTTPRequestor = edgeless_actor_class(
    id = "http_requestor",
    version = "0.1",
    outputs = [call_output("http_out", "edgeless.http.request", "edgeless.http.response")],
    inputs = [],
    inner_structure = [source("http_out")],
    code = file("http_requestor.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = HTTPRequestor
