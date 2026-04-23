# SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
load("../../functions/gpu_example/gpu.star", "GPUExample")

gpu_example = edgeless_actor(
    id = "pinger_i",
    klass = GPUExample,
    annotations = {}
)

wf = edgeless_workflow(
    "gpu_example",
    [gpu_example],
    annotations = {}
)

el_main = wf
