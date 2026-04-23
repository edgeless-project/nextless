# SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
# SPDX-License-Identifier: MIT
FileLog = edgeless_resource_class(
    id = "file-log",
    outputs = [],
    inputs = [call_input("line", "String")],
    inner_structure = [sink("line")],   
)

el_main = FileLog