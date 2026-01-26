// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use futures::FutureExt;

#[derive(Clone)]
struct MulticastWriter {
    sender: tokio::sync::mpsc::UnboundedSender<MulticastMessage>,
}

#[derive(Clone)]
pub struct MulticastLink {
    reader: std::sync::Arc<tokio::sync::Mutex<Vec<Box<dyn edgeless_api::link::LinkWriter>>>>,
    writer: Box<MulticastWriter>,
    _task: std::sync::Arc<tokio::sync::Mutex<MulticastTaskHandle>>,
}

struct MulticastTaskHandle(tokio::task::JoinHandle<()>);

#[derive(Clone)]
pub struct MulticastProvider {
    links: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<edgeless_api::link::LinkInstanceId, Box<MulticastLink>>>>,
}

struct MulticastMessage(Vec<u8>);

impl MulticastMessage {
    fn new(src: &edgeless_api::function_instance::InstanceId, data: &[u8]) -> Self {
        let mut v = Vec::with_capacity(16 * 2 + data.len());
        v.extend(data);
        v.extend(src.function_id.as_bytes());
        v.extend(src.node_id.as_bytes());
        Self(v)
    }

    fn parts(mut self) -> (edgeless_api::function_instance::InstanceId, Vec<u8>) {
        assert!(self.0.len() >= 32);
        let instance_id = edgeless_api::function_instance::InstanceId {
            node_id: uuid::Uuid::from_slice(&self.0[self.0.len() - 16..]).unwrap(),
            function_id: uuid::Uuid::from_slice(&self.0[self.0.len() - 32..self.0.len() - 16]).unwrap(),
        };
        self.0.resize(self.0.len() - 32, 0);
        (instance_id, self.0)
    }
}

impl MulticastLink {
    pub fn new(addr: std::net::Ipv4Addr, port: u16) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<MulticastMessage>();
        let reader: std::sync::Arc<tokio::sync::Mutex<Vec<Box<dyn edgeless_api::link::LinkWriter>>>> =
            std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let reader_clone = reader.clone();
        // TODO: Cleanup.
        let task = tokio::task::spawn(async move {
            let addr = addr;
            let sock_addr = std::net::SocketAddrV4::new(addr, port);
            let mut receiver = receiver;

            assert!(addr.is_multicast());

            // MacOS for some reason does not allow us to use a multicast address as the listen address:
            // https://stackoverflow.com/questions/49125176/cant-assign-requested-address-when-sending-to-a-udpsocket
            // https://stackoverflow.com/a/46489791
            // This is not ideal (we cannot use the same port for multiple connections).
            // I have not yet found a way to fix this. Windows also appears to have that behaviour:
            // https://stackoverflow.com/a/6219163
            #[cfg(target_os = "macos")]
            let bind_addr = std::net::SocketAddrV4::new(std::net::Ipv4Addr::UNSPECIFIED, port);
            #[cfg(not(target_os = "macos"))]
            let bind_addr = sock_addr.clone();

            // We need reuse_address, so we need to start with socket2:
            // https://github.com/tokio-rs/mio/issues/1426
            let sock = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::DGRAM, Some(socket2::Protocol::UDP)).unwrap();

            #[cfg(target_os = "macos")]
            sock.set_reuse_port(true).unwrap();

            // Allow testing on a single node:
            // https://stackoverflow.com/a/5340820
            sock.set_reuse_address(true).unwrap();
            sock.set_multicast_loop_v4(true).unwrap();

            // Required by Tokio.
            sock.set_nonblocking(true).unwrap();
            sock.bind(&socket2::SockAddr::from(std::net::SocketAddr::V4(bind_addr))).unwrap();

            // Convert to tokio: https://stackoverflow.com/a/77590253
            let sock = tokio::net::UdpSocket::from_std(std::net::UdpSocket::from(sock)).unwrap();

            sock.join_multicast_v4(addr, std::net::Ipv4Addr::UNSPECIFIED).unwrap();
            let mut buffer = vec![0_u8; 5000];

            loop {
                tokio::select! {
                    outgoing = Box::pin(receiver.recv()).fuse() => {
                        if let Some(outgoing) = outgoing {
                            sock.send_to(&outgoing.0[..], sock_addr).await.unwrap();
                        }
                    },
                    incomming = Box::pin(sock.recv_from(&mut buffer[..])).fuse() => {
                        match incomming {
                            Ok((data_size, _sender)) => {
                                for r in reader_clone.lock().await.iter_mut() {
                                    let data = MulticastMessage(Vec::from(&buffer[0..data_size]));
                                    let (src, data) = data.parts();
                                    r.handle(src, data).await;
                                }
                            },
                            Err(err) => {
                                log::error!("{err}");
                            },
                        }
                    }
                }
            }
        });

        MulticastLink {
            reader,
            writer: Box::new(MulticastWriter { sender }),
            _task: std::sync::Arc::new(tokio::sync::Mutex::new(MulticastTaskHandle(task))),
        }
    }
}

impl Default for MulticastProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MulticastProvider {
    pub fn new() -> Self {
        MulticastProvider {
            links: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }
}

impl Drop for MulticastTaskHandle {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkProvider for MulticastProvider {
    fn class(&self) -> edgeless_api::link::LinkType {
        edgeless_api::link::LinkType("MULTICAST".to_string())
    }

    async fn create(&mut self, req: edgeless_api::link::CreateLinkRequest) -> anyhow::Result<Box<dyn edgeless_api::link::LinkInstance>> {
        match self.links.lock().await.entry(req.id.clone()) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => Ok(occupied_entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                let cfg: crate::common::MulticastConfig = serde_json::from_slice(&req.config).unwrap();
                let link = Box::new(MulticastLink::new(cfg.ip, cfg.port));
                vacant_entry.insert(link.clone());
                return Ok(link);
            }
        }
    }
    async fn remove(&mut self, id: edgeless_api::link::LinkInstanceId) -> anyhow::Result<()> {
        self.links.lock().await.remove(&id);
        Ok(())
    }
    async fn register_reader(&mut self, link_id: &edgeless_api::link::LinkInstanceId, reader: Box<dyn edgeless_api::link::LinkWriter>) {
        self.links
            .lock()
            .await
            .get_mut(link_id)
            .unwrap()
            .as_mut()
            .reader
            .lock()
            .await
            .push(reader);
    }
    async fn get_writer(&mut self, link_id: &edgeless_api::link::LinkInstanceId) -> Option<Box<dyn edgeless_api::link::LinkWriter>> {
        Some(self.links.lock().await.get_mut(link_id).unwrap().as_mut().writer.clone())
    }
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkInstance for MulticastLink {
    async fn register_reader(&mut self, reader: Box<dyn edgeless_api::link::LinkWriter>) -> anyhow::Result<()> {
        self.reader.lock().await.push(reader);
        Ok(())
    }
    async fn get_writer(&mut self) -> Option<Box<dyn edgeless_api::link::LinkWriter>> {
        Some(self.writer.clone())
    }
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkWriter for MulticastWriter {
    async fn handle(&mut self, src: edgeless_api::function_instance::InstanceId, msg: Vec<u8>) {
        self.sender.send(MulticastMessage::new(&src, &msg[..])).unwrap();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn multicast_serialize_deserialize() {
        let data = vec![1, 2, 3, 4];
        let src = edgeless_api::function_instance::InstanceId {
            node_id: uuid::Uuid::new_v4(),
            function_id: uuid::Uuid::new_v4(),
        };

        let serialized = MulticastMessage::new(&src, &data[..]);

        let (deserialized_src, deserialized_data) = serialized.parts();

        assert_eq!(deserialized_src, src);
        assert_eq!(deserialized_data, data);
    }

    #[test]
    fn multicast_serialize_deserialize_empty_data() {
        let data = vec![];
        let src = edgeless_api::function_instance::InstanceId {
            node_id: uuid::Uuid::new_v4(),
            function_id: uuid::Uuid::new_v4(),
        };

        let serialized = MulticastMessage::new(&src, &data[..]);

        let (deserialized_src, deserialized_data) = serialized.parts();

        assert_eq!(deserialized_src, src);
        assert_eq!(deserialized_data, data);
    }
}
