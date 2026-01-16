load("../../functions/latency_harness/latency_harness.star", "LatencyHarness")
load("../../functions/basic_forwarder/basic_forwarder.star", "BasicForwarder")
load("./config.star", "fwd_id", "harness_id", "num_fwds", "fake_work_delay_ms", "inter_message_delay_ms")

N = num_fwds()

harness = edgeless_actor(
    id = "latency_harness_i",
    klass = LatencyHarness,
    annotations = {
        "scaling_mode": "singleton",
        "node_ids_allowed": harness_id(),
        "init-payload": ",".join([inter_message_delay_ms(), "1000", "0"])
    }
)

forwarders = [edgeless_actor(
    id = "forwarder{}_i".format(id),
    klass = BasicForwarder,
    annotations = {
        "node_ids_allowed": fwd_id(id),
        "scaling_mode": "singleton",
        "init-payload": fake_work_delay_ms()
    }
) for id in range(1, N+1) ]

harness.start >> forwarders[0].data_in

[forwarders[i].data_out >> forwarders[i+1].data_in for i in range(0, N - 1)]

forwarders[N-1].data_out >> harness.end

wf = edgeless_workflow(
    "circle_".format(N),
    forwarders + [harness],
    annotations = {}
)

el_main = wf
