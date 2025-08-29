BasicForwarder = edgeless_actor_class(
    id = "basic_forwarder",
    version = "0.1",
    outputs = [cast_output("data_out", "eft_eval_numbered_test_message")],
    inputs = [cast_input("data_in", "eft_eval_numbered_test_message")],
    inner_structure = [link("data_in", ["data_out"])],
    code = file("basic_forwarder.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = BasicForwarder
