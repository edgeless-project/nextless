LatencyHarness = edgeless_actor_class(
    id = "latency_harness",
    version = "0.1",
    outputs = [
        cast_output("start", "eft_eval_numbered_test_message")
    ],
    inputs = [
        cast_input("end", "eft_eval_numbered_test_message")
    ],
    inner_structure = [
       source("start"),
       sink("end")
    ],
    code = file("latency_harness.tar.gz"),
    code_type = "RUST_BASE"
)

el_main = LatencyHarness
