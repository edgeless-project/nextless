load("../../functions/mock_producer/producer.star", "MockProducer")
load("../../functions/mock_consumer/consumer.star", "MockConsumer")
load("../../functions/mock_broker/broker.star", "MockBroker")

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
        "node_id_match_any": id_str(id),
        "max_instances": "1",
    }
) for id in range(1, 5) ]

consumers = [edgeless_actor(
    id = "consumer{}_i".format(id),
    klass = MockConsumer,
    annotations = {
        "node_id_match_any": id_str(id),
        "max_instances": "5",
    }
) for id in range(5, 9)]

broker = edgeless_actor(
    id = "broker_i",
    klass = MockBroker,
    annotations = {
        "node_id_match_any": id_str(9),
        "max_instances": "1",
    }
)

[[getattr(producers[pid-1], "output{}".format(oid)) >> [getattr(broker, "input{}".format(oid))] for oid in range(1, 11)] for pid in range(1, 5)]
[getattr(broker, "output{}".format(x)) >> any([getattr(consumers[x-1], "input{}".format(x))]) for x in range(1, 5)]

wf = edgeless_workflow(
    "pub_sub_mock_baseline",
    consumers + producers + [broker],
    annotations = {}
)

el_main = wf
