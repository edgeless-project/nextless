MessagingTest = edgeless_actor_class(
    id = "messaging_test",
    version = "0.1",
    outputs = [cast_output("test_cast", "String"), call_output("test_call", "String")],
    inputs = [cast_input("test_cast_input", "String"), call_input("test_input_reply", "String", "String"), call_input("test_input_noreply", "String")],
    inner_structure = [
        link("test_cast_input", ["test_cast", "test_call"]),
        sink("test_call")
    ],
    code = file("messaging_test.wasm"),
    code_type = "RUST_WASM"
)

el_main = MessagingTest