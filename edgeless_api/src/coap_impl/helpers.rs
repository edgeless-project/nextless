// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use std::str::FromStr;

pub fn std_outputs_to_core_ouputs(
    output_mapping: &std::collections::HashMap<crate::function_instance::PortId, crate::common::Output>,
) -> anyhow::Result<heapless::Vec<(&str, edgeless_api_core::common::Output), 4>> {
    let mut outputs = heapless::Vec::<(&str, edgeless_api_core::common::Output), 4>::new();
    for (key, val) in output_mapping {
        outputs
            .push((
                &key.0,
                match val {
                    crate::common::Output::Single(instance_id, port_id) => {
                        edgeless_api_core::common::Output::Single(edgeless_api_core::common::Target {
                            instance_id: *instance_id,
                            port_id: edgeless_api_core::port::Port::<32>(heapless::String::<32>::from_str(&port_id.0).unwrap()),
                        })
                    }
                    crate::common::Output::Any(ids) => {
                        let mut id_vec = edgeless_api_core::common::TargetVec::<4>(heapless::Vec::new());
                        for (instance_id, port_id) in ids {
                            id_vec
                                .0
                                .push(edgeless_api_core::common::Target {
                                    instance_id: *instance_id,
                                    port_id: edgeless_api_core::port::Port::<32>(heapless::String::<32>::from_str(&port_id.0).unwrap()),
                                })
                                .unwrap();
                        }
                        edgeless_api_core::common::Output::Any(id_vec)
                    }
                    crate::common::Output::All(ids) => {
                        let mut id_vec = edgeless_api_core::common::TargetVec::<4>(heapless::Vec::new());
                        for (instance_id, port_id) in ids {
                            id_vec
                                .0
                                .push(edgeless_api_core::common::Target {
                                    instance_id: *instance_id,
                                    port_id: edgeless_api_core::port::Port::<32>(heapless::String::<32>::from_str(&port_id.0).unwrap()),
                                })
                                .unwrap();
                        }
                        edgeless_api_core::common::Output::Any(id_vec)
                    }
                    crate::common::Output::Link(_link_id) => todo!(),
                },
            ))
            .map_err(|_| anyhow::anyhow!("Too many outputs"))?;
    }
    Ok(outputs)
}
