# SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT


NODES = [
    # This will be dynamically generated
    "67fcab56-fea9-490d-9ae2-31713d51463b",
    "7e7b9ba6-0c4d-43c3-8ec1-8bac83c41ce8"
]


def fwd_id(id):
    return NODES[((id - 1) % len(NODES))]

def harness_id():
    return "67fcab56-fea9-490d-9ae2-31713d51463b"

def num_fwds():
    return 100

def inter_message_delay_ms():
    return "100"

def fake_work_delay_ms():
    return "0"
