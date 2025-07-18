BasicConsumer = edgeless_actor_class(
    id = "basic_consumer",
    version = "0.1",
    outputs = [],
    inputs = [cast_input("input_1", "test")],
    inner_structure = [
        sink("input_1")
    ],
    code = file("basic_consumer.tar.gz"),
    code_type = "RUST"
)

el_main = BasicConsumer
