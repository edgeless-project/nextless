// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use core::str::FromStr;

#[derive(Clone)]
pub struct EncodedFunctionInstanceSpecification<'a> {
    pub instance_id: crate::instance_id::InstanceId,
    pub class: EncodedFunctionClassSpecification,
    pub input_mapping: heapless::Vec<(&'a str, crate::common::Input), 4>,
    pub output_mapping: heapless::Vec<(&'a str, crate::common::Output), 4>,
}

#[derive(Clone)]
pub struct OwnedFunctionInstanceSpecification {
    pub instance_id: crate::instance_id::InstanceId,
    pub class: EncodedFunctionClassSpecification,
    pub input_mapping: heapless::Vec<(heapless::String<32>, crate::common::Input), 4>,
    pub output_mapping: heapless::Vec<(heapless::String<32>, crate::common::Output), 4>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EncodedFunctionClassSpecification {
    pub class_id: heapless::String<32>,
    pub class_type: heapless::String<32>,
    pub version: heapless::String<8>,
    pub image_hash: [u8; 32],
    // We might split this into id and size
    pub image_size: u64,
}

impl<C> minicbor::Encode<C> for EncodedFunctionInstanceSpecification<'_> {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.encode(self.instance_id)?;
        e.encode(&self.class)?;

        // e.encode()?;
        e.array(self.input_mapping.len().try_into().unwrap())?;
        for data in &self.input_mapping {
            e.encode(data)?;
        }

        e.array(self.output_mapping.len().try_into().unwrap())?;
        for data in &self.output_mapping {
            e.encode(data)?;
        }
        Ok(())
    }
}

impl<C> minicbor::CborLen<C> for EncodedFunctionInstanceSpecification<'_> {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        let mut size: usize = 0;

        size += self.instance_id.cbor_len(ctx);
        size += self.class.cbor_len(ctx);
        size += self.input_mapping[..self.input_mapping.len()].cbor_len(ctx);
        size += self.output_mapping[..self.output_mapping.len()].cbor_len(ctx);

        size
    }
}

impl<'b, C> minicbor::Decode<'b, C> for EncodedFunctionInstanceSpecification<'b> {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let instance_id = d.decode::<crate::instance_id::InstanceId>()?;
        let class = d.decode::<EncodedFunctionClassSpecification>()?;

        let mut input_mapping = heapless::Vec::<(&'b str, crate::common::Input), 4>::new();

        for item in d.array_iter::<(&str, crate::common::Input)>().unwrap() {
            if let Ok(item) = item {
                input_mapping.push(item).unwrap();
            }
        }

        let mut output_mapping = heapless::Vec::<(&'b str, crate::common::Output), 4>::new();

        for item in d.array_iter::<(&str, crate::common::Output)>().unwrap() {
            if let Ok(item) = item {
                output_mapping.push(item).unwrap();
            }
        }

        Ok(Self {
            instance_id,
            class,
            input_mapping,
            output_mapping,
        })
    }
}

impl<C> minicbor::Encode<C> for EncodedFunctionClassSpecification {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.encode(self.class_id.as_str())?;
        e.encode(self.class_type.as_str())?;
        e.encode(self.version.as_str())?;
        e.bytes(&self.image_hash)?;
        e.u64(self.image_size)?;
        Ok(())
    }
}

impl<C> minicbor::CborLen<C> for EncodedFunctionClassSpecification {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        self.class_id.cbor_len(ctx)
            + self.class_type.cbor_len(ctx)
            + self.version.cbor_len(ctx)
            + self.image_hash.cbor_len(ctx)
            + self.image_size.cbor_len(ctx)
    }
}

impl<'b, C> minicbor::Decode<'b, C> for EncodedFunctionClassSpecification {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let class_id: heapless::String<32> =
            heapless::String::<32>::from_str(d.str()?).map_err(|e| minicbor::decode::Error::message("Bad String"))?;
        let class_type: heapless::String<32> =
            heapless::String::<32>::from_str(d.str()?).map_err(|e| minicbor::decode::Error::message("Bad String"))?;
        let version: heapless::String<8> = heapless::String::<8>::from_str(d.str()?).map_err(|e| minicbor::decode::Error::message("Bad String"))?;
        let config_hash: [u8; 32] = d.bytes()?.try_into().map_err(|e| minicbor::decode::Error::message("Bad Hash"))?;
        let image_size: u64 = d.u64()?;
        Ok(EncodedFunctionClassSpecification {
            class_id,
            class_type,
            version,
            image_hash: config_hash,
            image_size,
        })
    }
}

impl EncodedFunctionInstanceSpecification<'_> {
    pub fn to_owned_spec(&self) -> OwnedFunctionInstanceSpecification {
        OwnedFunctionInstanceSpecification {
            instance_id: self.instance_id.clone(),
            class: self.class.clone(),
            input_mapping: self
                .input_mapping
                .iter()
                .map(|(k, v)| (heapless::String::from_str(k).unwrap(), v.clone()))
                .collect(),
            output_mapping: self
                .output_mapping
                .iter()
                .map(|(k, v)| (heapless::String::from_str(k).unwrap(), v.clone()))
                .collect(),
        }
    }

    pub fn from_owned_spec<'a>(owned: &'a OwnedFunctionInstanceSpecification) -> EncodedFunctionInstanceSpecification<'a> {
        EncodedFunctionInstanceSpecification {
            instance_id: owned.instance_id.clone(),
            class: owned.class.clone(),
            input_mapping: owned.input_mapping.iter().map(|(k, v)| (k.as_str(), v.clone())).collect(),
            output_mapping: owned.output_mapping.iter().map(|(k, v)| (k.as_str(), v.clone())).collect(),
        }
    }
}
