// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use crate::{function_instance::FunctionInstanceAPI, invocation::InvocationAPI};
use core::str::FromStr;

#[derive(Clone)]
pub struct EmbeddedAgent {
    own_node_id: edgeless_api_core::instance_id::NodeId,
    upstream_sender: embassy_sync::channel::Sender<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, AgentEvent, 2>,
    upstream_receiver: Option<embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, AgentEvent, 2>>,
    inner: &'static embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, EmbeddedAgentInner>,
    registration_signal: &'static embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, RegistrationReply>,
    internal_buffer_sender: embassy_sync::channel::Sender<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, StoredMessage, 2>,
    code_store: crate::code_store::CodeStore,
}

struct EmbeddedAgentInner {
    resources: &'static mut [&'static mut dyn crate::resource::ResourceDyn],
    wasm_runtime: Option<&'static mut crate::wasm_functions::WasmiRuntime>,
    delayed_start: Option<edgeless_api_core::function_instance::OwnedFunctionInstanceSpecification>,
}

struct AgentTask {
    inner: &'static embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, EmbeddedAgentInner>,
    internal_buffer_receiver: embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, StoredMessage, 2>,
}

#[embassy_executor::task]
pub async fn run_agent(slf: AgentTask) {
    loop {
        let event = slf.internal_buffer_receiver.receive().await;
        let mut lck = slf.inner.lock().await;
        for r in lck.resources.iter_mut() {
            if r.has_instance(&event.target).await {
                r.handle(event.clone()).await.unwrap();
                continue;
            }
        }
        if let Some(runtime) = &mut lck.wasm_runtime {
            if runtime.has_instance(&event.target).await {
                runtime.handle(event).await.unwrap();
            }
        }
    }
}

type StoredMessage = edgeless_api_core::invocation::Event;

pub enum AgentEvent {
    Invocation(StoredMessage),
    Registration(
        (
            edgeless_api_core::node_registration::EncodedNodeRegistration<'static>,
            &'static embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, RegistrationReply>,
        ),
    ),
    FetchImage {
        function_id: edgeless_api_core::instance_id::InstanceId,
        image_spec: edgeless_api_core::function_instance::EncodedFunctionClassSpecification,
    },
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
        cfg_if::cfg_if! {
            if #[cfg(feature = "alloc_static")] {
                let channel = alloc::boxed::Box::leak(alloc::boxed::Box::new(embassy_sync::channel::Channel::<
                    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                    AgentEvent,
                    2,
                >::new()));
                let buffer_channel = alloc::boxed::Box::leak(alloc::boxed::Box::new(embassy_sync::channel::Channel::<
                    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                    StoredMessage,
                    2,
                >::new()));
                let reply_channel = alloc::boxed::Box::leak(alloc::boxed::Box::new(embassy_sync::signal::Signal::<
                    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                    RegistrationReply,
                >::new()));
            } else {
                static CHANNEL_RAW: static_cell::StaticCell<embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, AgentEvent, 2>> =
                    static_cell::StaticCell::new();
                let channel = CHANNEL_RAW.init_with(embassy_sync::channel::Channel::<
                    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                    AgentEvent,
                    2,
                    >::new
                );
                static BUFFER_CHANNEL_RAW: static_cell::StaticCell<
                    embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, StoredMessage, 2>,
                > = static_cell::StaticCell::new();
                let buffer_channel =
                    BUFFER_CHANNEL_RAW.init_with(embassy_sync::channel::Channel::<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, StoredMessage, 2>::new);
                static REPLY_CHANNEL: static_cell::StaticCell<
                    embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, RegistrationReply>,
                > = static_cell::StaticCell::new();
                let reply_channel = REPLY_CHANNEL.init_with(embassy_sync::signal::Signal::<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, RegistrationReply>::new);
            }
        }

        let sender = channel.sender();
        let receiver = channel.receiver();
        let buffer_sender = buffer_channel.sender();
        let buffer_receiver = buffer_channel.receiver();

        cfg_if::cfg_if! {
            if #[cfg(feature = "alloc_static")] {
                let slf_inner = alloc::boxed::Box::leak(alloc::boxed::Box::new(embassy_sync::mutex::Mutex::new(
                    EmbeddedAgentInner {
                        resources: &mut resources[..],
                        wasm_runtime: runtime,
                        delayed_start: None,
                    },
                )));

                let slf = alloc::boxed::Box::leak(alloc::boxed::Box::new(EmbeddedAgent {
                    own_node_id: node_id,
                    upstream_sender: sender,
                    upstream_receiver: Some(receiver),
                    inner: slf_inner,
                    registration_signal: reply_channel,
                    internal_buffer_sender: buffer_sender,
                    code_store: crate::code_store::CodeStore::new(),
                }));
            } else {
                static SLF_INNER_RAW: static_cell::StaticCell<
                    embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, EmbeddedAgentInner>,
                > = static_cell::StaticCell::new();
                let slf_inner = SLF_INNER_RAW.init_with(|| {
                    embassy_sync::mutex::Mutex::new(EmbeddedAgentInner {
                        resources: &mut resources[..],
                        wasm_runtime: runtime,
                        delayed_start: None,
                    })
                });
                static SLF_RAW: static_cell::StaticCell<EmbeddedAgent> = static_cell::StaticCell::new();
                let slf = SLF_RAW.init_with(||EmbeddedAgent {
                    own_node_id: node_id,
                    upstream_sender: sender,
                    upstream_receiver: Some(receiver),
                    inner: slf_inner,
                    registration_signal: reply_channel,
                    internal_buffer_sender: buffer_sender,
                    code_store: crate::code_store::CodeStore::new(),
                });
            }
        }

        let agent_task = AgentTask {
            inner: slf_inner,
            internal_buffer_receiver: buffer_receiver,
        };
        spawner.spawn(run_agent(agent_task)).unwrap();

        {
            let mut lck = slf.inner.lock().await;
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
    ) -> Option<embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, AgentEvent, 2>> {
        self.upstream_receiver.take()
    }

    pub async fn register(&mut self, addr: embassy_net::Ipv4Address) {
        let mut url = heapless::String::<256>::new();
        let url_bytes = addr.octets();
        ufmt::uwrite!(url, "coap://{}.{}.{}.{}:7050", url_bytes[0], url_bytes[1], url_bytes[2], url_bytes[3]).unwrap();

        let lck = self.inner.lock().await;
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

    pub fn code_store(&self) -> crate::code_store::CodeStore {
        self.code_store.clone()
    }

    pub async fn fetch_complete(&mut self, _instance_id: edgeless_api_core::instance_id::InstanceId) {
        let delayed_start = {
            let mut lck = self.inner.lock().await;
            lck.delayed_start.take()
        };

        if let Some(delayed_start) = delayed_start {
            log::info!("Fetch Complete, Starting Function Now");
            let instace_spec = edgeless_api_core::function_instance::EncodedFunctionInstanceSpecification::from_owned_spec(&delayed_start);
            self.start_function(instace_spec).await.unwrap();
        }
    }
}

impl crate::invocation::InvocationAPI for EmbeddedAgent {
    async fn handle(&mut self, event: edgeless_api_core::invocation::Event) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        if event.target.node_id != self.own_node_id && event.source.node_id == self.own_node_id {
            self.upstream_sender.send(AgentEvent::Invocation(event)).await;
            Ok(edgeless_api_core::invocation::LinkProcessingResult::FINAL)
        } else {
            if self.internal_buffer_sender.try_send(event).is_ok() {
                return Ok(edgeless_api_core::invocation::LinkProcessingResult::PROCESSED);
            }
            Ok(edgeless_api_core::invocation::LinkProcessingResult::PASSED)
        }
    }
}

impl crate::resource_configuration::ResourceConfigurationAPI for EmbeddedAgent {
    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let mut lck = self.inner.lock().await;
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
        log::info!("R Start 1");
        let mut lck = self.inner.lock().await;
        log::info!("R Start 2");
        for r in lck.resources.iter_mut() {
            log::info!("R Start {} {}", instance_specification.class_type.len(), r.resource_class());
            if r.resource_class() == instance_specification.class_type {
                log::info!("Try Start");
                return r.start(instance_specification).await;
            }
        }
        log::info!("not found");
        Err(edgeless_api_core::common::ErrorResponse {
            summary: "ResourceProvider Not Found",
            detail: None,
        })
    }

    async fn patch<'a>(
        &mut self,
        patch_req: edgeless_api_core::resource_configuration::EncodedPatchRequest<'a>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let mut lck = self.inner.lock().await;
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
        if !self.code_store().has_image(&instance_specification.class).await {
            log::info!("Code Fetch Required");
            self.code_store.create_image(&instance_specification.class).await.unwrap();
            self.inner.lock().await.delayed_start = Some(instance_specification.to_owned_spec());
            self.upstream_sender
                .send(AgentEvent::FetchImage {
                    function_id: instance_specification.instance_id,
                    image_spec: instance_specification.class.clone(),
                })
                .await;
            return Ok(());
        }

        let mut lck = self.inner.lock().await;

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
        let mut lck = self.inner.lock().await;

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
        let mut lck = self.inner.lock().await;

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
