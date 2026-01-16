load("../../../functions/latency_harness/latency_harness.star", "LatencyHarness")
load("../../../functions/annotated_unused_output/annotated_unused_output.star", "AnnotatedUnusedOutput")

harness = edgeless_actor(
    id = "latency_harness_i",
    klass = LatencyHarness,
    annotations = {
        "scaling_mode": "singleton",
        "init-payload": ",".join(["100", "1000", "1000"])
    }
)

forwarder = edgeless_actor(
    id = "annotated_unused_output_i",
    klass = AnnotatedUnusedOutput,
    annotations = {
        "scaling_mode": "singleton",
    }
)

harness.start >> forwarder.data_in
forwarder.data_out >> harness.end
forwarder.unused_out >> harness.end

wf = edgeless_workflow(
    "unused_input_activation",
    [harness, forwarder],
    annotations = {}
)

el_main = wf
