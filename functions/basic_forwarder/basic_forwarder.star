BasicForwarder = edgeless_actor_class(
    id = "basic_forwarder",
    version = "0.1",
    outputs = [cast_output("out", "test_id")],
    inputs = [cast_input("in", "test_id")],
    inner_structure = [link("in", ["out"])],
    code = file("basic_forwarder.tar.gz"),
    code_type = "RUST"
)

el_main = BasicForwarder
