LatencyHarness = edgeless_actor_class(
    id = "latency_harness",
    version = "0.1",
    outputs = [
        cast_output("start", "test_id")
    ],
    inputs = [
        cast_input("end", "test_id")
    ],
    inner_structure = [
       source("start"),
       sink("end")
    ],
    code = file("latency_harness.tar.gz"),
    code_type = "RUST"
)

el_main = LatencyHarness
