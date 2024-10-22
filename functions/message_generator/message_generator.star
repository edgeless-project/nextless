MessageGenerator = edgeless_actor_class(
    id = "message_generator",
    version = "0.1",
    outputs = [call_output("message", "String")],
    inputs = [],
    inner_structure = [source("message")],
    code = file("message_generator.tar.gz"),
    code_type = "RUST"
)

el_main = MessageGenerator