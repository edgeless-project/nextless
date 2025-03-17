// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use crate::code_store::ImageEntry;
use crate::function_instance::FunctionInstanceAPI;
use crate::invocation::InvocationAPI;
use crate::resource_configuration::ResourceConfigurationAPI;

struct CoapMultiplexer {
    sock: embassy_net::udp::UdpSocket<'static>,
    out_reader: embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, crate::agent::AgentEvent, 2>,
    agent: crate::agent::EmbeddedAgent,
    app_buf_tx: &'static mut [u8; 2500],
    last_tokens: heapless::LinearMap<embassy_net::IpEndpoint, (u8, Option<Result<(), edgeless_api_core::common::ErrorResponse>>), 4>,
    peers: heapless::LinearMap<edgeless_api_core::node_registration::NodeId, embassy_net::IpEndpoint, 8>,
    token: u8,
    waiting_for_reply: Option<(
        u8,
        &'static embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, crate::agent::RegistrationReply>,
    )>,
    active_fetch: Option<ImageFetchJob>,
}

struct ImageFetchJob {
    spec: edgeless_api_core::function_instance::EncodedFunctionClassSpecification,
    requesting_function: edgeless_api_core::instance_id::InstanceId,
    current_offset: u64,
}

#[embassy_executor::task]
pub async fn coap_task(
    mut sock: embassy_net::udp::UdpSocket<'static>,
    out_reader: embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, crate::agent::AgentEvent, 2>,
    agent: crate::agent::EmbeddedAgent,
    rx_buffer: &'static mut [u8; 2500],
    tx_buffer: &'static mut [u8; 2500],
) {
    sock.bind(7050).unwrap();

    let mut slf = CoapMultiplexer {
        sock,
        out_reader,
        agent,
        app_buf_tx: tx_buffer,
        last_tokens: heapless::LinearMap::new(),
        peers: heapless::LinearMap::new(),
        token: 0,
        waiting_for_reply: None,
        active_fetch: None,
    };

    slf.task(rx_buffer).await;
}

impl CoapMultiplexer {
    async fn task(&mut self, rx_buffer: &'static mut [u8; 2500]) {
        loop {
            log::debug!("Receive Loop");
            let res = embassy_futures::select::select3(
                self.sock.recv_from(rx_buffer),
                self.out_reader.receive(),
                embassy_time::Timer::after_secs(1),
            )
            .await;

            match res {
                // External Message Received
                embassy_futures::select::Either3::First(res) => {
                    let (data_len, sender) = match res {
                        Ok(ret) => ret,
                        Err(err) => {
                            log::error!("UDP/COAP Receive Error: {:?}", err);
                            continue;
                        }
                    };
                    let (message, token) = match edgeless_api_core::coap_mapping::CoapDecoder::decode(&rx_buffer[..data_len]) {
                        Ok(ret) => ret,
                        Err(err) => {
                            log::error!("UDP/COAP Decode Error: {:?}", err);
                            continue;
                        }
                    };
                    let sender = sender.endpoint;
                    match message {
                        edgeless_api_core::coap_mapping::CoapMessage::Invocation(invocation) => {
                            self.incoming_invocation(sender, token, invocation).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::ResourceStart(start_spec) => {
                            self.incoming_resource_start(sender, token, start_spec).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::ResourceStop(stop_instance_id) => {
                            self.incoming_resource_stop(sender, token, stop_instance_id).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::ResourcePatch(patch_req) => {
                            self.incoming_resource_patch(sender, token, patch_req).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::PeerAdd((node_id, addr, port)) => {
                            self.incoming_peer_add(sender, token, node_id, &addr, port).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::PeerRemove(node_id) => {
                            self.incoming_peer_remove(sender, token, node_id).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::Response(data, success) => {
                            log::info!("Got Response: {}, {}", data.len(), success);
                            if let Some((t, channel)) = self.waiting_for_reply.take() {
                                if t == token {
                                    channel.signal(crate::agent::RegistrationReply::Sucess)
                                }
                            }
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::KeepAlive => {
                            self.incoming_keepalive(sender, token).await;
                        }
                        // #[cfg(feature = "wasm")]
                        edgeless_api_core::coap_mapping::CoapMessage::FunctionStart(start_spec) => {
                            log::info!("Is Start");
                            self.incoming_function_start(sender, token, start_spec).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::FunctionStop(stop_instance_id) => {
                            self.incoming_function_stop(sender, token, stop_instance_id).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::FunctionPatch(patch_req) => {
                            self.incoming_fucntion_patch(sender, token, patch_req).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::ResponseChunk { offset, size, buf } => {
                            let complete = if let Some(active_fetch) = &mut self.active_fetch {
                                let complete = offset + size >= active_fetch.spec.image_size;
                                log::trace!("Fetch Response: {} {} {} {}", offset, size, active_fetch.spec.image_size, complete);

                                let image = self.agent.code_store().get_image(&active_fetch.spec).await.unwrap();
                                image.update(offset as usize, buf, complete).unwrap();
                                if !complete {
                                    active_fetch.current_offset += size;
                                    self.fetch_next_chunk().await;
                                } else {
                                    self.agent.fetch_complete(active_fetch.requesting_function).await;
                                }
                                complete
                            } else {
                                false
                            };
                            if complete {
                                self.active_fetch = None;
                            }
                        }
                        _ => {
                            log::info!("Unhandled Message");
                        }
                    }
                }
                // Internal Message that needs to be sent out.
                embassy_futures::select::Either3::Second(event) => match event {
                    crate::agent::AgentEvent::Invocation(event) => {
                        self.outgoing_invocation(event).await;
                    }
                    crate::agent::AgentEvent::Registration((registration, reply_signal)) => {
                        self.outgoing_registration(&registration, reply_signal).await;
                    }
                    crate::agent::AgentEvent::FetchImage { function_id, image_spec } => {
                        if self.active_fetch.is_none() {
                            self.active_fetch = Some(ImageFetchJob {
                                spec: image_spec,
                                current_offset: 0,
                                requesting_function: function_id,
                            });
                            self.fetch_next_chunk().await;
                        } else {
                            log::error!("Parallel Fetch not Implemented");
                        }
                    }
                },
                // Periodic Retry
                embassy_futures::select::Either3::Third(_) => {
                    if self.active_fetch.is_some() {
                        self.fetch_next_chunk().await;
                    }
                }
            }
        }
    }

    async fn incoming_invocation(&mut self, sender: embassy_net::IpEndpoint, token: u8, invocation: edgeless_api_core::invocation::Event) {
        let key_entry = self.last_tokens.get_mut(&sender);
        match key_entry {
            None => {
                self.agent.handle(invocation).await.unwrap();
                if self.last_tokens.insert(sender, (token, None)).is_err() {
                    log::info!("Could not store token, duplicate delivery is possible!");
                }
            }
            // While we don't send back a response, we still need to block duplicate delivery.
            Some((entry, _message)) => {
                if &*entry < &token || token == 0 {
                    self.agent.handle(invocation).await.unwrap();
                    *entry = token;
                }
            }
        }
    }

    async fn incoming_resource_start<'a>(
        &mut self,
        sender: embassy_net::IpEndpoint,
        token: u8,
        start_spec: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) {
        let mut cloned_agent = self.agent.clone();
        self.at_most_once(sender, token, async move { cloned_agent.start(start_spec).await })
            .await
    }

    async fn incoming_resource_stop(
        &mut self,
        sender: embassy_net::IpEndpoint,
        token: u8,
        stop_instance_id: edgeless_api_core::instance_id::InstanceId,
    ) {
        let mut cloned_agent = self.agent.clone();
        self.at_most_once(sender, token, async move { cloned_agent.stop(stop_instance_id).await })
            .await
    }

    async fn incoming_resource_patch<'a>(
        &mut self,
        sender: embassy_net::IpEndpoint,
        token: u8,
        patch_req: edgeless_api_core::resource_configuration::EncodedPatchRequest<'a>,
    ) {
        let mut cloned_agent = self.agent.clone();
        self.at_most_once(sender, token, async move { cloned_agent.patch(patch_req).await }).await
    }

    async fn incoming_function_start<'a>(
        &'a mut self,
        sender: embassy_net::IpEndpoint,
        token: u8,
        start_spec: edgeless_api_core::function_instance::EncodedFunctionInstanceSpecification<'a>,
    ) {
        let mut cloned_agent = self.agent.clone();
        self.at_most_once(sender, token, async move { cloned_agent.start_function(start_spec).await })
            .await
    }

    async fn incoming_function_stop(
        &mut self,
        sender: embassy_net::IpEndpoint,
        token: u8,
        stop_instance_id: edgeless_api_core::instance_id::InstanceId,
    ) {
        let mut cloned_agent = self.agent.clone();
        self.at_most_once(sender, token, async move { cloned_agent.stop_function(stop_instance_id).await })
            .await
    }

    async fn incoming_fucntion_patch<'a>(
        &mut self,
        sender: embassy_net::IpEndpoint,
        token: u8,
        patch_req: edgeless_api_core::resource_configuration::EncodedPatchRequest<'a>,
    ) {
        let mut cloned_agent = self.agent.clone();
        self.at_most_once(sender, token, async move { cloned_agent.patch_function(patch_req).await })
            .await
    }

    async fn incoming_peer_add(&mut self, sender: embassy_net::IpEndpoint, token: u8, node_id: uuid::Uuid, addr: &[u8], port: u16) {
        log::info!("Got Peer Add {:?}, {}", addr, port);
        if self
            .peers
            .insert(
                edgeless_api_core::node_registration::NodeId(node_id),
                embassy_net::IpEndpoint {
                    addr: embassy_net::IpAddress::from(embassy_net::Ipv4Address::new(addr[0], addr[1], addr[2], addr[3])),
                    port,
                },
            )
            .is_err()
        {
            log::error!("Too many peers!");
        }
        let ((data, sender), _tail) =
            edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &[], token, self.app_buf_tx.as_mut_slice(), true);
        if let Err(err) = self.sock.send_to(data, sender).await {
            log::error!("UDP/COAP Send Error: {:?}", err);
        }
    }

    async fn incoming_peer_remove(&mut self, sender: embassy_net::IpEndpoint, token: u8, node_id: uuid::Uuid) {
        log::info!("Got Peer Remove");
        self.peers.remove(&edgeless_api_core::node_registration::NodeId(node_id));
        let ((data, sender), _tail) =
            edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &[], token, self.app_buf_tx.as_mut_slice(), true);
        if let Err(err) = self.sock.send_to(data, sender).await {
            log::error!("UDP/COAP Send Error: {:?}", err);
        }
    }

    async fn incoming_keepalive(&mut self, sender: embassy_net::IpEndpoint, token: u8) {
        log::info!("KeepAlive: {}", sender);
        let ((data, sender), _tail) =
            edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &[], token, self.app_buf_tx.as_mut_slice(), true);
        log::info!("KeepAlive 2");
        if let Err(err) = self.sock.send_to(data, sender).await {
            log::error!("keepalive UDP/COAP send error: {:?}", err);
        } else {
            log::info!("Sent Keepalive response");
        }
    }

    async fn outgoing_invocation(&mut self, event: edgeless_api_core::invocation::Event) {
        if let Some(peer) = self.peers.get(&edgeless_api_core::node_registration::NodeId(event.target.node_id)) {
            let ((data, endpoint), _tail) =
                edgeless_api_core::coap_mapping::COAPEncoder::encode_invocation_event(peer, event, self.token, self.app_buf_tx.as_mut_slice());
            self.token = match self.token {
                u8::MAX => 0,
                _ => self.token + 1,
            };
            if let Err(err) = self.sock.send_to(data, *endpoint).await {
                log::error!("UDP/COAP Send Error: {:?}", err);
            }
            // we don't wait for a reply here.
        }
    }

    async fn outgoing_registration(
        &mut self,
        registration: &edgeless_api_core::node_registration::EncodedNodeRegistration<'static>,
        reply_channel: &'static embassy_sync::signal::Signal<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            crate::agent::RegistrationReply,
        >,
    ) {
        let endpoint = crate::REGISTRATION_PEER;
        let ((data, endpoint), _tail) = edgeless_api_core::coap_mapping::COAPEncoder::encode_node_registration(
            endpoint,
            registration,
            self.token,
            self.app_buf_tx.as_mut_slice(),
        );
        let used_token = self.token;
        self.token = match self.token {
            u8::MAX => 0,
            _ => self.token + 1,
        };
        if let Err(err) = self.sock.send_to(data, endpoint).await {
            log::error!("UDP/COAP Send Error: {:?}", err);
        } else {
            self.waiting_for_reply = Some((used_token, reply_channel))
        }
    }

    async fn fetch_next_chunk(&mut self) {
        let endpoint = crate::REGISTRATION_PEER;
        if let Some(active_fetch) = &mut self.active_fetch {
            log::trace!("Fetch Next Chunk: {}", active_fetch.current_offset);
            let ((data, endpoint), _tail) = edgeless_api_core::coap_mapping::COAPEncoder::encode_fetch_image_chunk(
                endpoint,
                active_fetch.spec.image_hash,
                active_fetch.current_offset as usize,
                self.token,
                self.app_buf_tx.as_mut_slice(),
            );

            self.token = match self.token {
                u8::MAX => 0,
                _ => self.token + 1,
            };
            if let Err(err) = self.sock.send_to(data, endpoint).await {
                log::error!("UDP/COAP Send Error: {:?}", err);
            }
        }
    }

    async fn at_most_once<'a>(
        &'a mut self,
        sender: embassy_net::IpEndpoint,
        token: u8,
        operation: impl core::future::Future<Output = Result<(), edgeless_api_core::common::ErrorResponse>>,
    ) {
        let key_entry = self.last_tokens.get_mut(&sender);

        let ret = match key_entry {
            None => {
                let response = operation.await;
                if self.last_tokens.insert(sender, (token, Some(response.clone()))).is_err() {
                    log::info!("Could not store token, duplicate delivery is possible!");
                }
                Some(response)
            }
            Some((stored_token, stored_response)) => {
                if &*stored_token < &token || token == 0 {
                    let id = operation.await;
                    *stored_token = token;
                    *stored_response = Some(id.clone());

                    Some(id)
                } else if *stored_token == token {
                    stored_response.clone()
                } else {
                    None
                }
            }
        };

        if let Some(ret) = ret {
            let ((data, sender), _tail) = match ret {
                Ok(_) => edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &[], token, self.app_buf_tx.as_mut_slice(), true),
                Err(err) => {
                    let (data, tail) = edgeless_api_core::coap_mapping::COAPEncoder::encode_error_response(err, self.app_buf_tx.as_mut_slice());
                    edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, data, token, &mut tail[..], false)
                }
            };
            if let Err(err) = self.sock.send_to(data, sender).await {
                log::error!("UDP/COAP Send Error: {:?}", err);
            }
        }
    }
}
