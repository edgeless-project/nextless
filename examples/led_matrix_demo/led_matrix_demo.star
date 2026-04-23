# SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
load("../../functions/led_matrix_demo/led_matrix_demo.star", "LedMatrixDemo")
load("../../resources/led_matrix.star", "LedMatrix")

demo_producer_1 = edgeless_actor(
    id = "led_matrix_producer_1",
    klass = LedMatrixDemo,
    annotations = {
        # "scaling_mode": "all_nodes"
        "node_ids_allowed": "00000000-0000-0000-0000-000000000001"
    }
)

# demo_producer_2 = edgeless_actor(
#     id = "led_matrix_producer_2",
#     klass = LedMatrixDemo,
#     annotations = {
#         # "scaling_mode": "all_nodes"
#         "node_ids_allowed": "00000000-0000-0000-0000-000000000002"
#     }
# )

display_1 = edgeless_resource(
    id = "led-matrix-1-1",
    klass = LedMatrix,
    configurations = {},
    annotations = {
        # "scaling_mode": "all_nodes"
        "node_ids_allowed": "00000000-0000-0000-0000-000000000001"
    }
)

# display_2 = edgeless_resource(
#     id = "led-matrix-1-2",
#     klass = LedMatrix,
#     configurations = {},
#     annotations = {
#         # "scaling_mode": "all_nodes"
#         "node_ids_allowed": "00000000-0000-0000-0000-000000000002"
#     }
# )

demo_producer_1.value >> display_1.update
# demo_producer_2.value >> display_2.update

wf = edgeless_workflow(
    "led_matrix_test",
    [
        demo_producer_1, display_1,
       # demo_producer_2, display_2
    ],
    annotations = {}
)

el_main = wf
