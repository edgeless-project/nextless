load("../../functions/native_benefit_fwd/native_benefit_fwd.star", "NativeBenefitFwd")
load("../../functions/latency_harness/latency_harness.star", "LatencyHarness")


def id_str(id):
    unpadded = "%x" % (id)
    padded = ""
    for i in range(12 - len(unpadded)):
        padded = padded + "0"
    padded = padded + unpadded

    full = "00000000-0000-0000-0000-{}".format(padded)
    return full

harness = edgeless_actor(
    id = "latency_harness_i",
    klass = LatencyHarness,
    annotations = {
        "scaling_mode": "singleton",
        "init-payload": ",".join(["100", "1000", "0"]),
        "runtime_dialect_denied": "NATIVE_DYNAMIC",
    }
)

fwd = edgeless_actor(
    id  = "fwd",
    klass = NativeBenefitFwd,
    annotations = {
        "init-payload": "300",
        "node_ids_allowed": id_str(1),
    },
)

harness.start >> any([fwd.data_in])
fwd.data_out >> harness.end

wf = edgeless_workflow(
    "native_scaleup",
    [harness, fwd],
    annotations = {}
)

el_main = wf
