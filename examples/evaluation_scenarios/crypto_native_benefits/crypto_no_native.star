load("../../../functions/latency_harness/latency_harness.star", "LatencyHarness")
load("../../../functions/encryptor/encryptor.star", "Encryptor")
load("../../../functions/decryptor/decryptor.star", "Decryptor")

harness = edgeless_actor(
    id = "latency_harness_i",
    klass = LatencyHarness,
    annotations = {
        "scaling_mode": "singleton",
        "init-payload": ",".join(["100", "1000", "1000"]),
        "runtime_dialects_denied": "NATIVE_DYNAMIC",
    }
)

encryptor = edgeless_actor(
    id = "encryptor_i",
    klass = Encryptor,
    annotations = {
        "scaling_mode": "singleton",
        "runtime_dialects_denied": "NATIVE_DYNAMIC",
    }
)

decryptor = edgeless_actor(
    id = "decryptor_i",
    klass = Decryptor,
    annotations = {
        "scaling_mode": "singleton",
        "runtime_dialects_denied": "NATIVE_DYNAMIC",
    }
)

harness.start >> encryptor.data_in
encryptor.data_out >> decryptor.data_in
decryptor.data_out >> harness.end

wf = edgeless_workflow(
    "crypto_no_native",
    [harness, encryptor, decryptor],
    annotations = {}
)

el_main = wf
