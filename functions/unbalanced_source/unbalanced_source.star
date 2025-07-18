UnbalancedSource = edgeless_actor_class(
    id = "unbalanced_source",
    version = "0.1",
    outputs = [
        cast_output("frequent", "test"),
        cast_output("infrequent", "test")
    ],
    inputs = [],
    inner_structure = [
       source("frequent"),
       source("infrequent")
    ],
    code = file("unbalanced_source.tar.gz"),
    code_type = "RUST"
)

el_main = UnbalancedSource
