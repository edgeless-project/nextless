load("../../functions/latency_harness/latency_harness.star", "LatencyHarness")
load("../../functions/encryptor/encryptor.star", "Encryptor")
load("../../functions/decryptor/decryptor.star", "Decryptor")

harness = edgeless_actor(
    id = "latency_harness_i",
    klass = LatencyHarness,
    annotations = {
        "max_instances": "1",
        "init-payload": ",".join(["100", "1000", "1000"])
    }
)

encryptor = edgeless_actor(
    id = "encryptor_i",
    klass = Encryptor,
    annotations = {
        "max_instances": "1",
    }
)

decryptor = edgeless_actor(
    id = "decryptor_i",
    klass = Decryptor,
    annotations = {
        "max_instances": "1",
    }
)

harness.start >> encryptor.data_in
encryptor.data_out >> decryptor.data_in
decryptor.data_out >> harness.end

wf = edgeless_workflow(
    "crypto_allow_native",
    [harness, encryptor, decryptor],
    annotations = {}
)

el_main = wf
