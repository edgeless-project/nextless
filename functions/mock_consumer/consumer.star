MockConsumer = edgeless_actor_class(
    id = "mock_consumer",
    version = "0.1",
    outputs = [],
    inputs = [cast_input("input{}".format(x), "test") for x in range(1, 11)],
    inner_structure = [
        sink("input{}".format(x)) for x in range(1, 11)
    ],
    code = file("mock_consumer.tar.gz"),
    code_type = "RUST"
)

el_main = MockConsumer
