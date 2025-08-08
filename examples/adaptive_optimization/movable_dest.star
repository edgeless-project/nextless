load("../../functions/basic_consumer/basic_consumer.star", "BasicConsumer")
load("../../functions/unbalanced_source/unbalanced_source.star", "UnbalancedSource")

def id_str(id):
    unpadded = "%x" % (id)
    padded = ""
    for i in range(12 - len(unpadded)):
        padded = padded + "0"
    padded = padded + unpadded

    full = "00000000-0000-0000-0000-{}".format(padded)

    print(full)
    return full

source = edgeless_actor(
    id = "unbalanced_source",
    klass = UnbalancedSource,
    annotations = {
        "node_id_match_any": id_str(1),
        "max_instances": "1",
    }
)

frequent = edgeless_actor(
    id = "frequent_dest",
    klass = BasicConsumer,
    annotations = {
        "node_id_match_any": "{},{}".format(id_str(1), id_str(2)),
        "node_id_init_on": id_str(2),
        "max_instances": "1",
    }
)

infrequent = edgeless_actor(
    id = "infrequent_dest",
    klass = BasicConsumer,
    annotations = {
        "node_id_match_any": "{},{}".format(id_str(1), id_str(2)),
        "node_id_init_on": id_str(1),
        "max_instances": "1",
    }
)

source.frequent >> frequent.data_in
source.infrequent >> infrequent.data_in

wf = edgeless_workflow(
    "adaptive_move",
    [source, frequent, infrequent],
    annotations = {}
)

el_main = wf
