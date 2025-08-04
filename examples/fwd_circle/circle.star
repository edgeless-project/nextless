load("../../functions/latency_harness/latency_harness.star", "LatencyHarness")
load("../../functions/basic_forwarder/basic_forwarder.star", "BasicForwarder")
load("./config.star", "fwd_id", "harness_id", "num_fwds", "fake_work_delay_ms", "inter_message_delay_ms")

N = num_fwds()

harness = edgeless_actor(
    id = "latency_harness_i",
    klass = LatencyHarness,
    annotations = {
        "max_instances": "1",
        "node_id_match_any": harness_id(),
        "init-payload": inter_message_delay_ms()
    }
)

forwarders = [edgeless_actor(
    id = "forwarder{}_i".format(id),
    klass = BasicForwarder,
    annotations = {
        "node_id_match_any": fwd_id(id),
        "max_instances": "1",
        "init-payload": fake_work_delay_ms()
    }
) for id in range(1, N+1) ]

harness.start >> getattr(forwarders[0], "in")

[getattr(forwarders[i], "out") >> getattr(forwarders[i+1], "in") for i in range(0, N - 1)]

forwarders[N-1].out >> harness.end

wf = edgeless_workflow(
    "circle_10",
    forwarders + [harness],
    annotations = {}
)

el_main = wf
