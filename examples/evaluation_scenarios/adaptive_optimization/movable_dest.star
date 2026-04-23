# SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
load("../../../functions/basic_consumer/basic_consumer.star", "BasicConsumer")
load("../../../functions/unbalanced_source/unbalanced_source.star", "UnbalancedSource")

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
        "node_ids_allowed": id_str(1),
        "scaling_mode": "singleton",
    }
)

frequent = edgeless_actor(
    id = "frequent_dest",
    klass = BasicConsumer,
    annotations = {
        "node_ids_allowed": "{},{}".format(id_str(1), id_str(2)),
        "node_id_init_on": id_str(2),
        "scaling_mode": "singleton",
    }
)

infrequent = edgeless_actor(
    id = "infrequent_dest",
    klass = BasicConsumer,
    annotations = {
        "node_ids_allowed": "{},{}".format(id_str(1), id_str(2)),
        "node_id_init_on": id_str(1),
        "scaling_mode": "singleton",
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
