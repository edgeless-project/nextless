HTTPProcessor = edgeless_actor_class(
    id = "http_processor",
    version = "0.1",
    outputs = [],
    inputs = [call_input("new_req", "edgeless.http.Request", "edgeless.http.Response")],
    inner_structure = [sink("new_req")],
    code = file("http_processor.tar.gz"),
    code_type = "RUST"
)

el_main = HTTPProcessor