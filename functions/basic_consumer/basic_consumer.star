BasicConsumer = edgeless_actor_class(
    id = "basic_consumer",
    version = "0.1",
    outputs = [],
    inputs = [cast_input("data_in", "eft_eval_numbered_test_message")],
    inner_structure = [
        sink("data_in")
    ],
    code = file("basic_consumer.tar.gz"),
    code_type = "RUST"
)

el_main = BasicConsumer
