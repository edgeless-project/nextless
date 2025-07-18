// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use futures::StreamExt;
use opentelemetry::trace::TraceContextExt;
use opentelemetry::trace::Tracer;

use crate::core::*;
use crate::node_local::*;
use crate::remote_node::*;
use rand::seq::SliceRandom;

#[derive(Clone)]
struct IncommingLink {
    sender: tokio::sync::mpsc::UnboundedSender<DataplaneEvent>,
    #[allow(unused)]
    target_id: edgeless_api::function_instance::InstanceId,
    target_port: edgeless_api::function_instance::PortId,
    telemetry_handle: Option<Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>>,
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkWriter for IncommingLink {
    async fn handle(&mut self, src: edgeless_api::function_instance::InstanceId, msg: Vec<u8>) {
        if let Some(telemetry_handle) = &mut self.telemetry_handle {
            telemetry_handle.observe(
                edgeless_telemetry::telemetry_events::TelemetryEvent::MessageReceived(msg.len() as u64),
                std::collections::BTreeMap::from([
                    ("SOURCE_NODE_ID".to_string(), src.node_id.to_string()),
                    ("SOURCE_FUNCTION_ID".to_string(), src.function_id.to_string()),
                    ("SOURCE_PORT".to_string(), "UNKNOWN".to_string()),
                    ("DEST_PORT".to_string(), self.target_port.0.clone()),
                ]),
            );
        }
        self.sender
            .send(DataplaneEvent {
                source_id: edgeless_api::function_instance::InstanceId {
                    node_id: edgeless_api::function_instance::NODE_ID_NONE,
                    function_id: edgeless_api::function_instance::FUNCTION_ID_NONE,
                },
                source_port: edgeless_api::function_instance::PortId("UNKNOWN".to_string()),
                target_port: self.target_port.clone(),
                channel_id: 0,
                message: crate::core::Message::Cast(msg),
                context: opentelemetry::trace::SpanContext::empty_context(),
            })
            .unwrap();
    }
}

/// The main handle representing an element (identified by a `InstanceId`) across the dataplane.
/// The dataplane might require multiple links which are processed in a chain-like fashion.
#[derive(Clone)]
pub struct DataplaneHandle {
    alias_mapping: crate::alias_mapping::AliasMapping,
    slf: edgeless_api::function_instance::InstanceId,
    incomming_links: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<edgeless_api::link::LinkInstanceId, Box<IncommingLink>>>>,
    sender: tokio::sync::mpsc::UnboundedSender<DataplaneEvent>,
    receiver: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<DataplaneEvent>>>,
    link_manager: Box<dyn edgeless_api::link::LinkManager>,
    links: std::sync::Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<
                edgeless_api::link::LinkInstanceId,
                std::sync::Arc<tokio::sync::Mutex<Box<dyn edgeless_api::link::LinkWriter>>>,
            >,
        >,
    >,
    output_chain: std::sync::Arc<tokio::sync::Mutex<Vec<Box<dyn DataPlaneLink>>>>,
    receiver_overwrites: std::sync::Arc<tokio::sync::Mutex<TemporaryReceivers>>,
    next_id: u64,
    #[allow(unused)]
    telemetry_handle: Option<Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>>,
}

impl DataplaneHandle {
    async fn new(
        receiver_id: edgeless_api::function_instance::InstanceId,
        link_manager: Box<dyn edgeless_api::link::LinkManager>,
        output_chain: Vec<Box<dyn DataPlaneLink>>,
        receiver: futures::channel::mpsc::UnboundedReceiver<DataplaneEvent>,
        telemetry_handle: Option<Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>>,
    ) -> Self {
        let (main_sender, main_receiver) = tokio::sync::mpsc::unbounded_channel::<DataplaneEvent>();
        let receiver_overwrites = std::sync::Arc::new(tokio::sync::Mutex::new(TemporaryReceivers {
            temporary_receivers: std::collections::HashMap::new(),
        }));

        let clone_overwrites = receiver_overwrites.clone();
        // This task intercepts the messages received and routes responses towards temporary receivers while routing other events towards the main receiver used in `receive_next`.

        let cloned_sender = main_sender.clone();
        let mut cloned_telemetry = telemetry_handle.clone();

        tokio::spawn(async move {
            let mut receiver = receiver;
            loop {
                if let Some(DataplaneEvent {
                    source_id,
                    source_port,
                    channel_id,
                    message,
                    target_port,
                    context,
                }) = receiver.next().await
                {
                    if let Some(sender) = clone_overwrites.lock().await.temporary_receivers.remove(&channel_id) {
                        match sender.send((source_id, message.clone())) {
                            Ok(_) => {
                                continue;
                            }
                            Err(_) => {
                                log::error!("Tried to use expired overwrite send handle.");
                            }
                        }
                    }
                    if let Some(telemetry_handle) = &mut cloned_telemetry {
                        telemetry_handle.observe(
                            edgeless_telemetry::telemetry_events::TelemetryEvent::MessageReceived(message.payload_len() as u64),
                            std::collections::BTreeMap::from([
                                ("SOURCE_NODE_ID".to_string(), source_id.node_id.to_string()),
                                ("SOURCE_FUNCTION_ID".to_string(), source_id.function_id.to_string()),
                                ("SOURCE_PORT".to_string(), source_port.0.clone()),
                                ("DEST_PORT".to_string(), target_port.0.clone()),
                            ]),
                        );
                    }
                    match cloned_sender.send(DataplaneEvent {
                        source_id,
                        source_port,
                        channel_id,
                        message,
                        target_port,
                        context,
                    }) {
                        Ok(_) => {}
                        Err(_) => {
                            break;
                        }
                    }
                }
            }
        });

        DataplaneHandle {
            alias_mapping: crate::alias_mapping::AliasMapping::new(),
            slf: receiver_id,
            sender: main_sender.clone(),
            incomming_links: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            receiver: std::sync::Arc::new(tokio::sync::Mutex::new(main_receiver)),
            output_chain: std::sync::Arc::new(tokio::sync::Mutex::new(output_chain)),
            link_manager,
            links: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            receiver_overwrites,
            next_id: 1,
            telemetry_handle,
        }
    }

    /// Main receive function for receiving the next cast or call event.
    /// This is NOT used for processing replies to return values.
    pub async fn receive_next(&mut self) -> DataplaneEvent {
        loop {
            // log::info!("Q: {}", self.receiver.lock().await.len());
            if let Some(DataplaneEvent {
                source_id,
                source_port,
                channel_id,
                message,
                target_port: target_channel,
                context,
            }) = self.receiver.lock().await.recv().await
            {
                if std::mem::discriminant(&message) == std::mem::discriminant(&Message::Cast(Vec::new()))
                    || std::mem::discriminant(&message) == std::mem::discriminant(&Message::Call(Vec::new()))
                {
                    return DataplaneEvent {
                        source_id,
                        source_port,
                        channel_id,
                        message,
                        target_port: target_channel,
                        context,
                    };
                }
                log::error!("Unprocesses other message {message:?}");
            }
        }
    }

    pub async fn update_mapping(
        &mut self,
        new_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Input>,
        new_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Output>,
    ) {
        log::debug!("Got Update: Outputs: {new_output_mapping:?}; Inputs: {new_input_mapping:?}");
        let ((removed_inputs, removed_outputs), (added_inputs, added_outputs)) =
            self.alias_mapping.update(new_input_mapping, new_output_mapping).await;
        log::debug!(
            "Update Processed: RemovedI: {removed_inputs:?}; RemovedO: {removed_outputs:?}; AddedI: {added_inputs:?}, AddedO: {added_outputs:?}"
        );

        for (added_i_id, i) in added_inputs {
            if let edgeless_api::common::Input::Link(l) = i {
                self.add_incomming_link(edgeless_api::function_instance::PortId(added_i_id), &l).await;
            }
        }

        for (added_o_id, o) in added_outputs {
            if let edgeless_api::common::Output::Link(l) = o {
                self.add_outgoing_link(edgeless_api::function_instance::PortId(added_o_id), &l).await;
            }
        }
    }

    async fn add_incomming_link(&mut self, port: edgeless_api::function_instance::PortId, link_id: &edgeless_api::link::LinkInstanceId) {
        let incomming_link = Box::new(IncommingLink {
            sender: self.sender.clone(),
            target_id: self.slf,
            target_port: port,
            telemetry_handle: self.telemetry_handle.clone(),
        });

        self.link_manager.register_reader(link_id, incomming_link.clone()).await.unwrap();

        self.incomming_links.lock().await.insert(link_id.clone(), incomming_link);
    }

    async fn add_outgoing_link(&mut self, _port: edgeless_api::function_instance::PortId, link_id: &edgeless_api::link::LinkInstanceId) {
        let link = self.link_manager.get_writer(link_id).await;
        if let Some(link) = link {
            self.links
                .lock()
                .await
                .insert(link_id.clone(), std::sync::Arc::new(tokio::sync::Mutex::new(link)));
            log::info!("Added Link: {}", link_id.0);
        } else {
            log::error!("Could not get Requested Link");
        }
    }

    pub async fn send_alias(&mut self, target: String, msg: &[u8], context: opentelemetry::Context) -> anyhow::Result<()> {
        let call_handler_span = opentelemetry::global::tracer("dataplane").start_with_context(format!("send_{target}"), &context);
        let context = opentelemetry::Context::current_with_span(call_handler_span);
        if target == "self" {
            self.send_inner(
                self.slf,
                Message::Cast(msg.to_vec()),
                edgeless_api::function_instance::PortId("INTERNAL".to_string()),
                edgeless_api::function_instance::PortId("INTERNAL".to_string()),
                0,
                context,
            )
            .await;
            Ok(())
        } else if let Some(target_ouput) = self.alias_mapping.get_mapping(&target).await {
            match target_ouput {
                edgeless_api::common::Output::Single(instance_id, port_id) => {
                    self.send_inner(
                        instance_id,
                        Message::Cast(msg.to_vec()),
                        port_id.clone(),
                        edgeless_api::function_instance::PortId(target.clone()),
                        0,
                        context.clone(),
                    )
                    .await;
                }
                edgeless_api::common::Output::Any(ids) => {
                    let id = ids.choose(&mut rand::thread_rng());
                    if let Some((instance_id, port_id)) = id {
                        self.send_inner(
                            *instance_id,
                            Message::Cast(msg.to_vec()),
                            port_id.clone(),
                            edgeless_api::function_instance::PortId(target.clone()),
                            0,
                            context.clone(),
                        )
                        .await;
                    } else {
                        return Err(anyhow::anyhow!("Unknown Alias"));
                    }
                }
                edgeless_api::common::Output::All(ids) => {
                    for (instance_id, port_id) in ids {
                        self.send_inner(
                            instance_id,
                            Message::Cast(msg.to_vec()),
                            port_id.clone(),
                            edgeless_api::function_instance::PortId(target.clone()),
                            0,
                            context.clone(),
                        )
                        .await;
                    }
                }
                edgeless_api::common::Output::Link(link_id) => {
                    self.send_to_link(&link_id, msg.to_vec()).await;
                }
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Unknown Alias"))
        }
    }

    pub async fn call_alias(&mut self, alias: String, msg: &[u8], context: opentelemetry::Context) -> CallRet {
        let call_handler_span = opentelemetry::global::tracer("dataplane").start_with_context(format!("call_{alias}"), &context);
        let context = opentelemetry::Context::current_with_span(call_handler_span);
        if alias == "self" {
            self.call_raw(self.slf, edgeless_api::function_instance::PortId("INTERNAL".to_string()), msg, context)
                .await
            // return Ok(self.data_plane.call(self.instance_id.clone(), msg.to_string()).await);
        } else if let Some(target) = self.alias_mapping.get_mapping(&alias).await {
            // return self.call_raw(target, msg).await;
            match target {
                edgeless_api::common::Output::Single(instance_id, port_id) => {
                    // self.data_plane.send(id, msg.to_string()).await;
                    return self.call_raw(instance_id, port_id, msg, context).await;
                }
                edgeless_api::common::Output::Any(ids) => {
                    let id = ids.choose(&mut rand::thread_rng());
                    if let Some((instance_id, port_id)) = id {
                        // self.data_plane.send(id.clone(), msg.to_string()).await;
                        return self.call_raw(*instance_id, port_id.clone(), msg, context).await;
                    } else {
                        // return Err(GuestAPIError::UnknownAlias);
                        CallRet::Err
                    }
                }
                edgeless_api::common::Output::All(_ids) => {
                    // TODO(raphaelhetzel) introduce new error for this
                    // return Err(GuestAPIError::UnknownAlias);
                    CallRet::Err
                }
                edgeless_api::common::Output::Link(_) => CallRet::Err,
            }
        } else {
            log::warn!("Unknown alias.");
            // Err(GuestAPIError::UnknownAlias)
            CallRet::Err
        }
    }

    pub async fn send_to_link(&mut self, link_id: &edgeless_api::link::LinkInstanceId, msg: Vec<u8>) {
        if let Some(link) = self.links.lock().await.get(link_id) {
            link.lock().await.handle(self.slf, msg).await;
        } else {
            log::info!("Link not found: {}", link_id.0);
        }
    }

    /// Send a `cast` event.
    pub async fn send(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        target_port: edgeless_api::function_instance::PortId,
        msg: &[u8],
        context: opentelemetry::Context,
    ) {
        let call_handler_span = opentelemetry::global::tracer("dataplane").start_with_context("send", &context);
        let context = opentelemetry::Context::current_with_span(call_handler_span);
        self.send_inner(
            target,
            Message::Cast(msg.to_vec()),
            target_port,
            edgeless_api::function_instance::PortId("UNKNOWN".to_string()),
            0,
            context,
        )
        .await;
    }

    // Send a `call` event and wait for the return event.
    // Internally, this sets up a receiver override to handle the message before it would be sent to the `receive_next` function.
    pub async fn call(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        target_port: edgeless_api::function_instance::PortId,
        msg: &[u8],
        context: opentelemetry::Context,
    ) -> CallRet {
        let call_handler_span = opentelemetry::global::tracer("dataplane").start_with_context("call", &context);
        let context = opentelemetry::Context::current_with_span(call_handler_span);
        self.call_raw(target, target_port, msg, context).await
    }

    pub async fn call_raw(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        target_port: edgeless_api::function_instance::PortId,
        msg: &[u8],
        context: opentelemetry::Context,
    ) -> CallRet {
        let (sender, receiver) = futures::channel::oneshot::channel::<(edgeless_api::function_instance::InstanceId, Message)>();
        let channel_id = self.next_id;
        self.next_id += 1;
        // Potential Leak: This is only received if a message is received (or the handle is dropped)
        self.receiver_overwrites.lock().await.temporary_receivers.insert(channel_id, sender);
        self.send_inner(
            target,
            Message::Call(msg.to_vec()),
            target_port,
            edgeless_api::function_instance::PortId("UNKNOWN".to_string()),
            channel_id,
            context.clone(),
        )
        .await;
        match receiver.await {
            Ok((_src, msg)) => match msg {
                Message::CallRet(ret) => CallRet::Reply(ret),
                Message::CallNoRet => CallRet::NoReply,
                _ => CallRet::Err,
            },
            Err(_) => CallRet::Err,
        }
    }

    // Reply to a `call` event using the `channel_id` used to send the request.
    pub async fn reply(&mut self, target: edgeless_api::function_instance::InstanceId, channel_id: u64, msg: CallRet) {
        self.send_inner(
            target,
            match msg {
                CallRet::Reply(msg) => Message::CallRet(msg),
                CallRet::NoReply => Message::CallNoRet,
                CallRet::Err => Message::Err,
            },
            edgeless_api::function_instance::PortId("UNKNOWN".to_string()),
            edgeless_api::function_instance::PortId("UNKNOWN".to_string()),
            channel_id,
            opentelemetry::Context::new(),
        )
        .await;
    }

    async fn send_inner(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        msg: Message,
        target_port: edgeless_api::function_instance::PortId,
        source_port: edgeless_api::function_instance::PortId,
        channel_id: u64,
        context: opentelemetry::Context,
    ) {
        let mut lck = self.output_chain.lock().await;
        for link in &mut lck.iter_mut() {
            if link
                .handle_send(
                    &target,
                    msg.clone(),
                    &self.slf,
                    channel_id,
                    target_port.clone(),
                    source_port.clone(),
                    context.span().span_context().clone(),
                )
                .await
                == LinkProcessingResult::FINAL
            {
                return;
            }
        }
        log::info!("Unprocessed Message: {:?} {:?}->{:?}", msg, self.slf, target);
    }
}

struct TemporaryReceivers {
    temporary_receivers: std::collections::HashMap<u64, futures::channel::oneshot::Sender<(edgeless_api::function_instance::InstanceId, Message)>>,
}

#[derive(Clone)]
pub struct DataplaneProvider {
    local_provider: std::sync::Arc<tokio::sync::Mutex<NodeLocalLinkProvider>>,
    remote_provider: std::sync::Arc<tokio::sync::Mutex<RemoteLinkProvider>>,
    link_manager: std::sync::Arc<tokio::sync::Mutex<LinkManager>>,
}

struct LinkManager {
    link_providers: std::collections::HashMap<edgeless_api::link::LinkProviderId, Box<dyn edgeless_api::link::LinkProvider>>,
    links: std::collections::HashMap<edgeless_api::link::LinkInstanceId, Box<dyn edgeless_api::link::LinkInstance>>,
}

impl LinkManager {
    fn new() -> Self {
        LinkManager {
            link_providers: std::collections::HashMap::new(),
            links: std::collections::HashMap::new(),
        }
    }

    fn providers(&self) -> Vec<(edgeless_api::link::LinkType, edgeless_api::link::LinkProviderId)> {
        self.link_providers.iter().map(|(p_id, p)| (p.class(), p_id.clone())).collect()
    }
}

impl DataplaneProvider {
    pub async fn new(node_id: uuid::Uuid, invocation_url: String, invocation_url_coap: Option<String>) -> Self {
        let remote_provider = std::sync::Arc::new(tokio::sync::Mutex::new(RemoteLinkProvider::new(node_id).await));

        let (_, _, port) = edgeless_api::util::parse_http_host(&invocation_url.clone()).unwrap();

        let clone_provider = remote_provider.clone();
        let _server = tokio::spawn(edgeless_api::grpc_impl::invocation::InvocationAPIServer::run(
            clone_provider.lock().await.incomming_api().await,
            invocation_url,
        ));

        if let Some(invocation_url_coap) = invocation_url_coap {
            let (_, coap_ip, coap_port) = edgeless_api::util::parse_http_host(&invocation_url_coap.clone()).unwrap();
            log::info!("Start COAP Invocation Server {coap_ip}:{port}");

            let _coap_server = tokio::spawn(edgeless_api::coap_impl::invocation::CoapInvocationServer::run(
                clone_provider.lock().await.incomming_api().await,
                std::net::SocketAddrV4::new(coap_ip.parse().unwrap(), coap_port),
            ));
        }

        let mut lm = LinkManager::new();
        lm.link_providers.insert(
            edgeless_api::link::LinkProviderId(uuid::Uuid::new_v4()),
            Box::new(edgeless_link_multicast::link::MulticastProvider::new()),
        );

        Self {
            local_provider: std::sync::Arc::new(tokio::sync::Mutex::new(NodeLocalLinkProvider::new())),
            remote_provider,
            link_manager: std::sync::Arc::new(tokio::sync::Mutex::new(lm)),
        }
    }

    pub async fn get_handle_for(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        telemetry_handle: Option<Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>>,
    ) -> DataplaneHandle {
        let (sender, receiver) = futures::channel::mpsc::unbounded::<DataplaneEvent>();
        let output_chain = vec![
            self.local_provider.lock().await.new_link(target, sender.clone()).await,
            self.remote_provider.lock().await.new_link(target, sender.clone()).await,
        ];
        DataplaneHandle::new(
            target,
            edgeless_api::link::LinkManagerClone::clone_box(self),
            output_chain,
            receiver,
            telemetry_handle,
        )
        .await
    }

    pub async fn add_peer(&mut self, peer: EdgelessDataplanePeerSettings) {
        log::debug!("add_peer {peer:?}");
        self.remote_provider
            .lock()
            .await
            .add_peer(peer.node_id, Self::connect_peer(&peer).await)
            .await;
    }

    pub async fn del_peer(&mut self, node_id: uuid::Uuid) {
        log::debug!("del_peer {node_id:?}");
        self.remote_provider.lock().await.del_peer(node_id).await;
    }

    async fn connect_peer(target: &EdgelessDataplanePeerSettings) -> Box<dyn edgeless_api::invocation::InvocationAPI> {
        let (proto, url, port) = edgeless_api::util::parse_http_host(&target.invocation_url).unwrap();
        match proto {
            edgeless_api::util::Proto::COAP => {
                Box::new(edgeless_api::coap_impl::CoapClient::new(std::net::SocketAddrV4::new(url.parse().unwrap(), port)).await)
            }
            _ => Box::new(edgeless_api::grpc_impl::invocation::InvocationAPIClient::new(&target.invocation_url).await),
        }
    }

    pub async fn link_providers(&self) -> Vec<(edgeless_api::link::LinkType, edgeless_api::link::LinkProviderId)> {
        self.link_manager.lock().await.providers()
    }
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkManager for DataplaneProvider {
    async fn register_reader(
        &mut self,
        link_id: &edgeless_api::link::LinkInstanceId,
        reader: Box<dyn edgeless_api::link::LinkWriter>,
    ) -> anyhow::Result<()> {
        if let Some(link) = self.link_manager.lock().await.links.get_mut(link_id) {
            link.register_reader(reader).await.unwrap();
            return Ok(());
        }
        return Err(anyhow::anyhow!("Link not Found: {}", link_id.0));
    }

    async fn get_writer(&mut self, link_id: &edgeless_api::link::LinkInstanceId) -> Option<Box<dyn edgeless_api::link::LinkWriter>> {
        if let Some(link) = self.link_manager.lock().await.links.get_mut(link_id) {
            return link.get_writer().await;
        }
        None
    }
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkInstanceAPI for DataplaneProvider {
    async fn create(&mut self, req: edgeless_api::link::CreateLinkRequest) -> anyhow::Result<()> {
        log::info!("Create Link Called");

        let mut lm = self.link_manager.lock().await;
        if let Some(link_provider) = lm.link_providers.get_mut(&req.provider) {
            let link = link_provider.create(req.clone()).await?;
            lm.links.insert(req.id, link);
            log::info!("Added Link");

            return Ok(());
        }
        Err(anyhow::anyhow!("Not Possible"))
    }

    async fn remove(&mut self, id: edgeless_api::link::LinkInstanceId) -> anyhow::Result<()> {
        let mut lm = self.link_manager.lock().await;

        lm.links.remove(&id);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::handle::*;

    #[tokio::test]
    async fn local_normal_path() {
        let node_id = uuid::Uuid::new_v4();
        let fid_1 = edgeless_api::function_instance::InstanceId::new(node_id);
        let fid_2 = edgeless_api::function_instance::InstanceId::new(node_id);

        let mut provider = DataplaneProvider::new(node_id, "http://127.0.0.1:7096".to_string(), None).await;

        let mut handle_1 = provider.get_handle_for(fid_1, None).await;
        let mut handle_2 = provider.get_handle_for(fid_2, None).await;

        handle_1
            .send(
                fid_2,
                edgeless_api::function_instance::PortId("test".to_string()),
                "Test".as_bytes(),
                opentelemetry::Context::new(),
            )
            .await;

        let res = handle_2.receive_next().await;
        assert_eq!(
            std::mem::discriminant(&res.message),
            std::mem::discriminant(&crate::core::Message::Cast(Vec::new()))
        );
    }

    #[tokio::test]
    async fn local_call_with_return() {
        let node_id = uuid::Uuid::new_v4();
        let fid_1 = edgeless_api::function_instance::InstanceId::new(node_id);
        let fid_2 = edgeless_api::function_instance::InstanceId::new(node_id);

        let mut provider = DataplaneProvider::new(node_id, "http://127.0.0.1:7097".to_string(), None).await;

        let mut handle_1 = provider.get_handle_for(fid_1, None).await;
        let mut handle_2 = provider.get_handle_for(fid_2, None).await;

        let return_handle = tokio::spawn(async move {
            handle_1
                .call(
                    fid_2,
                    edgeless_api::function_instance::PortId("test".to_string()),
                    "Test".as_bytes(),
                    opentelemetry::Context::new(),
                )
                .await
        });

        let req = handle_2.receive_next().await;
        assert_eq!(
            std::mem::discriminant(&req.message),
            std::mem::discriminant(&crate::core::Message::Call(Vec::new()))
        );

        handle_2.reply(req.source_id, req.channel_id, CallRet::NoReply).await;

        let repl = return_handle.await.unwrap();
        assert_eq!(std::mem::discriminant(&CallRet::NoReply), std::mem::discriminant(&repl));
    }

    #[tokio::test]
    async fn grpc_impl_e2e() {
        let node_id = uuid::Uuid::new_v4();
        let node_id_2 = uuid::Uuid::new_v4();
        let fid_1 = edgeless_api::function_instance::InstanceId::new(node_id);
        let fid_2 = edgeless_api::function_instance::InstanceId::new(node_id_2);

        let provider1_f = tokio::spawn(async move {
            let mut dataplane = DataplaneProvider::new(node_id, "http://127.0.0.1:7099".to_string(), None).await;
            dataplane
                .add_peer(EdgelessDataplanePeerSettings {
                    node_id: node_id_2,
                    invocation_url: "http://127.0.0.1:7098".to_string(),
                })
                .await;
            dataplane
        });

        let provider2_f = tokio::spawn(async move {
            let mut dataplane = DataplaneProvider::new(node_id_2, "http://127.0.0.1:7098".to_string(), None).await;
            dataplane
                .add_peer(EdgelessDataplanePeerSettings {
                    node_id,
                    invocation_url: "http://127.0.0.1:7099".to_string(),
                })
                .await;
            dataplane
        });

        // This test got stuck during initial testing. I suspect that this was due to the use of common ports across the testsuite
        // but the timeouts should prevent it from blocking the entire testsuite if that was not the reason (timeout will lead to failure).
        let (provider_1_r, provider_2_r) = futures::join!(
            tokio::time::timeout(tokio::time::Duration::from_secs(5), provider1_f),
            tokio::time::timeout(tokio::time::Duration::from_secs(5), provider2_f)
        );
        let mut provider_1 = provider_1_r.unwrap().unwrap();
        let mut provider_2 = provider_2_r.unwrap().unwrap();

        let mut handle_1 = provider_1.get_handle_for(fid_1, None).await;
        let mut handle_2 = provider_2.get_handle_for(fid_2, None).await;

        handle_1
            .send(
                fid_2,
                edgeless_api::function_instance::PortId("test".to_string()),
                "Test".as_bytes(),
                opentelemetry::Context::new(),
            )
            .await;
        let cast_req = handle_2.receive_next().await;
        assert_eq!(
            std::mem::discriminant(&cast_req.message),
            std::mem::discriminant(&crate::core::Message::Cast(Vec::new()))
        );

        let cloned_id_1 = fid_1;
        let mut cloned_handle_2 = handle_2.clone();

        let return_handle = tokio::spawn(async move {
            cloned_handle_2
                .call(
                    cloned_id_1,
                    edgeless_api::function_instance::PortId("test".to_string()),
                    "Test".as_bytes(),
                    opentelemetry::Context::new(),
                )
                .await
        });

        let call_req = handle_1.receive_next().await;
        assert_eq!(
            std::mem::discriminant(&call_req.message),
            std::mem::discriminant(&crate::core::Message::Call(Vec::new()))
        );
        handle_1.reply(call_req.source_id, call_req.channel_id, CallRet::NoReply).await;

        let repl = return_handle.await.unwrap();
        assert_eq!(std::mem::discriminant(&CallRet::NoReply), std::mem::discriminant(&repl));
    }
}
