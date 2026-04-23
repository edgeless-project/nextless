# SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
load("../../../functions/mock_producer/producer.star", "MockProducer")
load("../../../functions/mock_consumer/consumer.star", "MockConsumer")

def id_str(id):
    unpadded = "%x" % (id)
    padded = ""
    for i in range(12 - len(unpadded)):
        padded = padded + "0"
    padded = padded + unpadded

    full = "00000000-0000-0000-0000-{}".format(padded)

    print(full)
    return full

producers = [edgeless_actor(
    id = "producer{}_i".format(id),
    klass = MockProducer,
    annotations = {
        "node_ids_allowed": id_str(id),
        "scaling_mode": "singleton",
    }
) for id in range(1, 5) ]

consumers = [edgeless_actor(
    id = "consumer{}_i".format(id),
    klass = MockConsumer,
    annotations = {
        "scaling_mode": "scalable",
        "node_ids_allowed": id_str(id),
        "max_instances": "5",
    }
) for id in range(5, 9)]

[
    [
        getattr(producers[pid-1], "output{}".format(oid)) >> [getattr(consumers[oid-1], "input{}".format(oid))]
        for oid in range(1, 5)
    ]
    for pid in range(1, 5)
]


wf = edgeless_workflow(
    "multicast_direct_no_multi",
    consumers + producers,
    annotations = {
        "DISABLE_MULTICAST": "1",
    }
)

el_main = wf
