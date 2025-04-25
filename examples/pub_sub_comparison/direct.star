load("../../functions/mock_producer/producer.star", "MockProducer")
load("../../functions/mock_consumer/consumer.star", "MockConsumer")

producer1 = edgeless_actor(
    id = "producer1_i",
    klass = MockProducer,
    annotations = {
        "max_instances": "1",
    }
)

consumers = [edgeless_actor(
    id = "consumer{}_i".format(id),
    klass = MockConsumer,
    annotations = {
        "max_instances": "1",
    }
) for id in range(1, 6)]

[getattr(producer1, "output{}".format(x)) << topic("topic{}".format(x)) for x in range(1, 11)]
[getattr(consumers[x-1], "input{}".format(x)) << topic("topic{}".format(x)) for x in range(1, 6)]

wf = edgeless_workflow(
    "pub_sub_mock_baseline",
    consumers + [producer1],
    annotations = {}
)

el_main = wf
