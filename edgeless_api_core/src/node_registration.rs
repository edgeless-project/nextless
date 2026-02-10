// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use core::str::FromStr;

#[derive(Clone, PartialEq, Eq)]
pub struct NodeId(pub uuid::Uuid);

#[derive(Clone)]
pub struct EncodedNodeRegistration<'a> {
    pub node_id: NodeId,
    pub agent_url: heapless::String<256>,
    pub invocation_url: heapless::String<256>,
    pub resources: heapless::Vec<ResourceProviderSpecification<'a>, 4>,
    pub runtimes: heapless::Vec<EncodedRuntimeType, 4>, // 4: node capabilities
}

#[derive(Clone)]
pub struct EncodedRuntimeType {
    pub base_type: heapless::String<32>,
    pub features: heapless::Vec<heapless::String<32>, 4>,
}

#[derive(Clone)]
pub struct ResourceProviderSpecification<'a> {
    pub provider_id: &'a str,
    pub class_type: &'a str,
    pub outputs: heapless::Vec<&'a str, 4>,
}

impl<C> minicbor::Encode<C> for NodeId {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        let n_id = *self.0.as_bytes();
        e.bytes(&n_id)?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for NodeId {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let n_id: [u8; 16] = (*d.bytes()?).try_into().unwrap();
        Ok(NodeId(uuid::Uuid::from_bytes(n_id)))
    }
}

impl<C> minicbor::CborLen<C> for NodeId {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        let n_id = *self.0.as_bytes();
        n_id.cbor_len(ctx)
    }
}

impl<C> minicbor::Encode<C> for EncodedNodeRegistration<'_> {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        let mut e = e;
        e = e.encode(self.node_id.clone())?;
        e = e.encode(self.agent_url.as_str())?;
        e = e.encode(self.invocation_url.as_str())?;

        {
            e = e.array(self.resources.len().try_into().unwrap())?;
            for spec in &self.resources {
                e = e.encode(spec)?;
            }
        }
        {
            e = e.array(self.runtimes.len().try_into().unwrap())?;
            for rt in &self.runtimes {
                e = e.encode(rt)?;
            }
        }

        Ok(())
    }
}

impl<C> minicbor::Encode<C> for EncodedRuntimeType {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        let mut e = e;
        e = e.encode(self.base_type.as_str())?;

        {
            e = e.array(self.features.len().try_into().unwrap())?;
            for spec in &self.features {
                e = e.encode(spec.as_str())?;
            }
        }

        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for EncodedNodeRegistration<'b> {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let id: NodeId = d.decode()?;
        let agent_url: &str = d.str()?;
        let invocation_url: &str = d.str()?;

        let mut resources: heapless::Vec<ResourceProviderSpecification, 4> = heapless::Vec::new();

        for item in d.array_iter::<ResourceProviderSpecification<'b>>()?.flatten() {
            if resources.push(item).is_err() {
                log::error!("Too many Resources");
            }
        }

        let mut runtimes = heapless::Vec::<EncodedRuntimeType, 4>::new();
        for item in d.array_iter::<EncodedRuntimeType>()?.flatten() {
            if runtimes.push(item).is_err() {
                log::error!("Too many Runtimes");
            }
        }

        Ok(EncodedNodeRegistration {
            node_id: id,
            agent_url: heapless::String::from_str(agent_url).unwrap(),
            invocation_url: heapless::String::from_str(invocation_url).unwrap(),
            resources,
            runtimes,
        })
    }
}

impl<'b, C> minicbor::Decode<'b, C> for EncodedRuntimeType {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let base_type: &str = d.str()?;

        let mut features = heapless::Vec::<heapless::String<32>, 4>::new();
        for item in d.array_iter::<&'b str>()?.flatten() {
            if features
                .push(heapless::String::from_str(item).map_err(|e| minicbor::decode::Error::message("String Failure: {e:?}"))?)
                .is_err()
            {
                log::error!("Too many Runtime Features");
            }
        }

        Ok(EncodedRuntimeType {
            base_type: heapless::String::from_str(base_type).unwrap(),
            features,
        })
    }
}

impl<C> minicbor::CborLen<C> for EncodedNodeRegistration<'_> {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        let mut len = self.node_id.cbor_len(ctx) + self.agent_url.cbor_len(ctx) + self.invocation_url.cbor_len(ctx);

        len += self.resources[..self.resources.len()].cbor_len(ctx);
        len + self.runtimes[..self.runtimes.len()].cbor_len(ctx)
    }
}

impl<C> minicbor::CborLen<C> for EncodedRuntimeType {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        let len = self.base_type.cbor_len(ctx);

        let fts: heapless::Vec<&str, 4> = self.features.iter().map(|i| i.as_str()).collect();

        len + fts[..fts.len()].cbor_len(ctx)
    }
}

impl<C> minicbor::Encode<C> for ResourceProviderSpecification<'_> {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        let mut e = e;
        e = e.encode(self.provider_id)?;
        e = e.encode(self.class_type)?;

        {
            e = e.array(self.outputs.len().try_into().unwrap())?;
            for output in &self.outputs {
                e = e.encode(output)?;
            }
        }

        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for ResourceProviderSpecification<'b> {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let provider_id: &str = d.decode()?;
        let class_type: &str = d.decode()?;

        let mut outputs = heapless::Vec::new();
        for item in d.array_iter::<&str>()?.flatten() {
            outputs.push(item).unwrap();
        }

        Ok(ResourceProviderSpecification {
            provider_id,
            class_type,
            outputs,
        })
    }
}

impl<C> minicbor::CborLen<C> for ResourceProviderSpecification<'_> {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        let len = self.provider_id.cbor_len(ctx) + self.class_type.cbor_len(ctx);

        let mut data: [&str; 4] = [""; 4];
        let mut data_count = 0;

        for i in &self.outputs {
            data[data_count] = i;
            data_count += 1;
        }

        len + data[..data_count].cbor_len(ctx)
    }
}
