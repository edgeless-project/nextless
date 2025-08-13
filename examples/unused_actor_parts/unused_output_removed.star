load("../../functions/latency_harness/latency_harness.star", "LatencyHarness")
load("../../functions/unused_output/unused_output.star", "UnusedOutput")

harness = edgeless_actor(
    id = "latency_harness_i",
    klass = LatencyHarness,
    annotations = {
        "max_instances": "1",
        "init-payload": ",".join(["100", "1000", "1000"])
    }
)

forwarder = edgeless_actor(
    id = "unused_output_i",
    klass = UnusedOutput,
    annotations = {
        "max_instances": "1",
    }
)

harness.start >> forwarder.data_in
forwarder.data_out >> harness.end

wf = edgeless_workflow(
    "unused_input_activation",
    [harness, forwarder],
    annotations = {}
)

el_main = wf
