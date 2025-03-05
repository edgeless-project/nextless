use core::str::FromStr;

// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
#[derive(Clone)]
pub struct EmbeddedAgent {
    own_node_id: edgeless_api_core::instance_id::NodeId,
    upstream_sender: embassy_sync::channel::Sender<'static, embassy_sync::blocking_mutex::raw::NoopRawMutex, AgentEvent, 2>,
    upstream_receiver: Option<embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::NoopRawMutex, AgentEvent, 2>>,
    inner: &'static core::cell::RefCell<embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, EmbeddedAgentInner>>,
    registration_signal: &'static embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::NoopRawMutex, RegistrationReply>,
    internal_buffer_sender: embassy_sync::channel::Sender<'static, embassy_sync::blocking_mutex::raw::NoopRawMutex, StoredMessage, 2>,
}

struct EmbeddedAgentInner {
    resources: &'static mut [&'static mut dyn crate::resource::ResourceDyn],
    wasm_runtime: Option<&'static mut crate::wasm_functions::WasmiRuntime>,
    internal_buffer_receiver: embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::NoopRawMutex, StoredMessage, 2>,
}

type StoredMessage = edgeless_api_core::invocation::Event;

pub enum AgentEvent {
    Invocation(StoredMessage),
    Registration(
        (
            edgeless_api_core::node_registration::EncodedNodeRegistration<'static>,
            &'static embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::NoopRawMutex, RegistrationReply>,
        ),
    ),
}

pub enum RegistrationReply {
    Sucess,
    Failure,
}

impl EmbeddedAgent {
    pub async fn new(
        spawner: embassy_executor::Spawner,
        node_id: edgeless_api_core::instance_id::NodeId,
        runtime: Option<&'static mut crate::wasm_functions::WasmiRuntime>,
        resources: &'static mut [&'static mut dyn crate::resource::ResourceDyn],
    ) -> &'static mut EmbeddedAgent {
        static CHANNEL_RAW: static_cell::StaticCell<embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::NoopRawMutex, AgentEvent, 2>> =
            static_cell::StaticCell::new();
        let channel = CHANNEL_RAW.init_with(embassy_sync::channel::Channel::<embassy_sync::blocking_mutex::raw::NoopRawMutex, AgentEvent, 2>::new);

        let sender = channel.sender();
        let receiver = channel.receiver();

        static BUFFER_CHANNEL_RAW: static_cell::StaticCell<
            embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::NoopRawMutex, StoredMessage, 2>,
        > = static_cell::StaticCell::new();
        let buffer_channel =
            BUFFER_CHANNEL_RAW.init_with(embassy_sync::channel::Channel::<embassy_sync::blocking_mutex::raw::NoopRawMutex, StoredMessage, 2>::new);

        let buffer_sender = buffer_channel.sender();
        let buffer_receiver = buffer_channel.receiver();

        static SLF_INNER_RAW: static_cell::StaticCell<
            core::cell::RefCell<embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, EmbeddedAgentInner>>,
        > = static_cell::StaticCell::new();
        let slf_inner = SLF_INNER_RAW.init_with(|| {
            core::cell::RefCell::new(embassy_sync::mutex::Mutex::new(EmbeddedAgentInner {
                resources: &mut resources[..],
                wasm_runtime: runtime,
                internal_buffer_receiver: buffer_receiver,
            }))
        });

        static REPLY_CHANNEL: static_cell::StaticCell<
            embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::NoopRawMutex, RegistrationReply>,
        > = static_cell::StaticCell::new();

        static SLF_RAW: static_cell::StaticCell<EmbeddedAgent> = static_cell::StaticCell::new();
        let slf = SLF_RAW.init_with(|| EmbeddedAgent {
            own_node_id: node_id,
            upstream_sender: sender,
            upstream_receiver: Some(receiver),
            inner: slf_inner,
            registration_signal: REPLY_CHANNEL
                .init_with(embassy_sync::signal::Signal::<embassy_sync::blocking_mutex::raw::NoopRawMutex, RegistrationReply>::new),
            internal_buffer_sender: buffer_sender,
        });

        {
            let inner = slf.inner.borrow_mut();
            let mut lck = inner.lock().await;
            for r in lck.resources.iter_mut() {
                r.launch(spawner, slf.clone()).await;
            }
            if let Some(runtime) = &mut lck.wasm_runtime {
                runtime.launch(slf.clone()).await;
            }
        }

        slf
    }

    pub fn upstream_receiver(
        &mut self,
    ) -> Option<embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::NoopRawMutex, AgentEvent, 2>> {
        self.upstream_receiver.take()
    }

    pub async fn register(&mut self, addr: embassy_net::Ipv4Address) {
        let mut url = heapless::String::<256>::new();
        let url_bytes = addr.octets();
        ufmt::uwrite!(url, "coap://{}.{}.{}.{}:7050", url_bytes[0], url_bytes[1], url_bytes[2], url_bytes[3]).unwrap();

        let tmp = self.inner.borrow_mut();
        let lck = tmp.lock().await;
        let mut resources = heapless::Vec::new();
        for i in &lck.resources[..] {
            let mut outputs = heapless::Vec::new();

            for j in i.outputs() {
                if outputs.push(*j).is_err() {
                    log::error!("Resource has too many outputs!");
                }
            }

            if resources
                .push(edgeless_api_core::node_registration::ResourceProviderSpecification {
                    provider_id: i.provider_id(),
                    class_type: i.resource_class(),
                    outputs,
                })
                .is_err()
            {
                log::error!("Node has to many resources!");
            }
        }

        let reg = edgeless_api_core::node_registration::EncodedNodeRegistration {
            node_id: edgeless_api_core::node_registration::NodeId(self.own_node_id),
            agent_url: url.clone(),
            invocation_url: url,
            resources,
            runtimes: heapless::Vec::from_slice(&[heapless::String::from_str("RUST_WASM").unwrap()]).unwrap(),
        };

        loop {
            self.registration_signal.reset();
            self.upstream_sender
                .send(AgentEvent::Registration((reg.clone(), self.registration_signal)))
                .await;
            if let RegistrationReply::Sucess = self.registration_signal.wait().await {
                return;
            }
        }
    }
}

impl crate::invocation::InvocationAPI for EmbeddedAgent {
    async fn handle(&mut self, event: edgeless_api_core::invocation::Event) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        if event.target.node_id != self.own_node_id && event.source.node_id == self.own_node_id {
            self.upstream_sender.send(AgentEvent::Invocation(event)).await;
            Ok(edgeless_api_core::invocation::LinkProcessingResult::FINAL)
        } else {
            let inner = self.inner.try_borrow_mut();

            if let Ok(inner) = inner {
                let mut lck = inner.lock().await;

                let mut handled = false;

                for r in lck.resources.iter_mut() {
                    if r.has_instance(&event.target).await {
                        r.handle(event.clone()).await;
                        handled = true;
                    }
                }

                if !handled {
                    if let Some(runtime) = &mut lck.wasm_runtime {
                        if runtime.has_instance(&event.target).await {
                            runtime.handle(event).await;
                        }
                    }
                }

                loop {
                    if let Ok(event) = lck.internal_buffer_receiver.try_receive() {
                        for r in lck.resources.iter_mut() {
                            if r.has_instance(&event.target).await {
                                return r.handle(event).await;
                            }
                        }

                        if let Some(runtime) = &mut lck.wasm_runtime {
                            if runtime.has_instance(&event.target).await {
                                runtime.handle(event).await;
                            }
                        }
                    } else {
                        break;
                    }
                }
            } else {
                if let Ok(_) = self.internal_buffer_sender.try_send(event) {
                    return Ok(edgeless_api_core::invocation::LinkProcessingResult::PROCESSED);
                } else {
                }
            }
            Ok(edgeless_api_core::invocation::LinkProcessingResult::PASSED)
        }
    }
}

impl crate::resource_configuration::ResourceConfigurationAPI for EmbeddedAgent {
    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let inner = self.inner.borrow_mut();
        let mut lck = inner.lock().await;
        for r in lck.resources.iter_mut() {
            if r.has_instance(&resource_id).await {
                return r.stop(resource_id).await;
            }
        }
        Err(edgeless_api_core::common::ErrorResponse {
            summary: "ResourceProvider Not Found",
            detail: None,
        })
    }

    async fn start<'a>(
        &mut self,
        instance_specification: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let inner = self.inner.borrow_mut();
        let mut lck = inner.lock().await;
        for r in lck.resources.iter_mut() {
            if r.resource_class() == instance_specification.class_type {
                return r.start(instance_specification).await;
            }
        }
        Err(edgeless_api_core::common::ErrorResponse {
            summary: "ResourceProvider Not Found",
            detail: None,
        })
    }

    async fn patch<'a>(
        &mut self,
        patch_req: edgeless_api_core::resource_configuration::EncodedPatchRequest<'a>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let inner = self.inner.borrow_mut();
        let mut lck = inner.lock().await;
        let mut my_patch = patch_req;

        my_patch.instance_id = edgeless_api_core::instance_id::InstanceId {
            node_id: self.own_node_id,
            function_id: my_patch.instance_id.function_id,
        };
        for r in lck.resources.iter_mut() {
            if r.has_instance(&my_patch.instance_id).await {
                return r.patch(my_patch).await;
            }
        }
        Err(edgeless_api_core::common::ErrorResponse {
            summary: "ResourceProvider Not Found",
            detail: None,
        })
    }
}

impl crate::function_instance::FunctionInstanceAPI for EmbeddedAgent {
    async fn start_function<'a>(
        &mut self,
        instance_specification: edgeless_api_core::function_instance::EncodedFunctionInstanceSpecification<'a>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let inner = self.inner.borrow_mut();
        let mut lck = inner.lock().await;

        if let Some(runtime) = &mut lck.wasm_runtime {
            log::info!("Agent Function Start");
            runtime.start_function(instance_specification).await
        } else {
            Err(edgeless_api_core::common::ErrorResponse {
                summary: "No Runtime",
                detail: None,
            })
        }
    }

    async fn stop_function(
        &mut self,
        instance_id: edgeless_api_core::instance_id::InstanceId,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let inner = self.inner.borrow_mut();
        let mut lck = inner.lock().await;

        if let Some(runtime) = &mut lck.wasm_runtime {
            runtime.stop_function(instance_id).await
        } else {
            Err(edgeless_api_core::common::ErrorResponse {
                summary: "No Runtime",
                detail: None,
            })
        }
    }

    async fn patch_function<'a>(
        &mut self,
        patch_req: edgeless_api_core::resource_configuration::EncodedPatchRequest<'a>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let inner = self.inner.borrow_mut();
        let mut lck = inner.lock().await;

        if let Some(runtime) = &mut lck.wasm_runtime {
            runtime.patch_function(patch_req).await
        } else {
            Err(edgeless_api_core::common::ErrorResponse {
                summary: "No Runtime",
                detail: None,
            })
        }
    }
}
