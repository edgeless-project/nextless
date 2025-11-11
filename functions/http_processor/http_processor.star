HTTPProcessor = edgeless_actor_class(
    id = "http_processor",
    version = "0.1",
    outputs = [cast_output("log_value", "String")],
    inputs = [call_input("new_req", "edgeless.http.Request", "edgeless.http.Response")],
    inner_structure = [sink("new_req"), source("log_value")],
    code = file("http_processor.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = HTTPProcessor
