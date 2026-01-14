load("../../../functions/basic_consumer/basic_consumer.star", "BasicConsumer")
load("../../../functions/unbalanced_source/unbalanced_source.star", "UnbalancedSource")
load("../../../functions/basic_forwarder/basic_forwarder.star", "BasicForwarder")

source = edgeless_actor(
    id = "source",
    klass = UnbalancedSource,
    annotations = {
        "max_instances": "1",
    }
)

sink = edgeless_actor(
    id = "sink",
    klass = BasicConsumer,
    annotations = {
        "max_instances": "1",
    }
)

used_forwarders = [edgeless_actor(
    id = "used_{}".format(id),
    klass = BasicForwarder,
    annotations = {
        "max_instances": "1",
    }
) for id in range(0, 10) ]

unused_forwarders = [edgeless_actor(
    id = "unused_{}".format(id),
    klass = BasicForwarder,
    annotations = {
        "max_instances": "1",
    }
) for id in range(0, 10) ]

source.frequent >> used_forwarders[0].data_in
source.infrequent >> unused_forwarders[0].data_in

[used_forwarders[i].data_out >> used_forwarders[i+1].data_in for i in range(0, 9)]
used_forwarders[9].data_out >> sink.data_in

[unused_forwarders[i].data_out >> unused_forwarders[i+1].data_in for i in range(0, 9)]

wf = edgeless_workflow(
    "unused_chain",
    used_forwarders + unused_forwarders + [source, sink],
    annotations = {}
)

el_main = wf
