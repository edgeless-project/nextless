# SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
load("../../../functions/latency_harness/latency_harness.star", "LatencyHarness")
load("../../../functions/encryptor/encryptor.star", "Encryptor")
load("../../../functions/decryptor/decryptor.star", "Decryptor")

harness = edgeless_actor(
    id = "latency_harness_i",
    klass = LatencyHarness,
    annotations = {
        "init-payload": ",".join(["100", "1000", "1000"]),
        "scaling_mode": "singleton",
    }
)

encryptor = edgeless_actor(
    id = "encryptor_i",
    klass = Encryptor,
    annotations = {
        "scaling_mode": "singleton",
    }
)

decryptor = edgeless_actor(
    id = "decryptor_i",
    klass = Decryptor,
    annotations = {
        "scaling_mode": "singleton",
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
