load("../../functions/mock_producer/producer.star", "MockProducer")
load("../../functions/mock_consumer/consumer.star", "MockConsumer")
load("../../functions/mock_broker/broker.star", "MockBroker")

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

broker = edgeless_actor(
    id = "broker_i",
    klass = MockBroker,
    annotations = {
        "max_instances": "1",
    }
)

[getattr(producer1, "output{}".format(x)) >> any([getattr(broker, "input{}".format(x))]) for x in range(1, 11)]
[getattr(broker, "output{}".format(x)) >> any([getattr(consumers[x-1], "input{}".format(x))]) for x in range(1, 6)]

wf = edgeless_workflow(
    "pub_sub_mock_baseline",
    consumers + [producer1, broker],
    annotations = {}
)

el_main = wf
