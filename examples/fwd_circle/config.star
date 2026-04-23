# SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT


NUM_NODES = 10

def id_str(id):
    unpadded = "%x" % (id)
    padded = ""
    for i in range(12 - len(unpadded)):
        padded = padded + "0"
    padded = padded + unpadded

    full = "00000000-0000-0000-0000-{}".format(padded)

    print(full)
    return full


def fwd_id(id):
    return id_str((id - 1) % NUM_NODES + 1)

def harness_id():
    return id_str(1)

def num_fwds():
    return 100

def inter_message_delay_ms():
    return "100"

def fake_work_delay_ms():
    return "0"
