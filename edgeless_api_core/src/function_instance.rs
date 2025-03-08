use core::str::FromStr;

#[derive(Clone)]
pub struct EncodedFunctionInstanceSpecification<'a> {
    pub instance_id: crate::instance_id::InstanceId,
    pub class: EncodedFunctionClassSpecification<'a>,
    pub input_mapping: heapless::Vec<(&'a str, crate::common::Input), 4>,
    pub output_mapping: heapless::Vec<(&'a str, crate::common::Output), 4>,
}

#[derive(Clone)]
pub struct EncodedFunctionClassSpecification<'a> {
    pub class_id: heapless::String<32>,
    pub class_type: heapless::String<32>,
    pub version: heapless::String<8>,
    pub code: &'a [u8],
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

impl<C> minicbor::Encode<C> for EncodedFunctionClassSpecification<'_> {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.encode(self.class_id.as_str())?;
        e.encode(self.class_type.as_str())?;
        e.encode(self.version.as_str())?;
        e.bytes(self.code)?;
        Ok(())
    }
}

impl<C> minicbor::CborLen<C> for EncodedFunctionClassSpecification<'_> {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        self.class_id.cbor_len(ctx) + self.class_type.cbor_len(ctx) + self.version.cbor_len(ctx) + self.code.cbor_len(ctx)
    }
}

impl<'b, C> minicbor::Decode<'b, C> for EncodedFunctionClassSpecification<'b> {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let class_id: heapless::String<32> =
            heapless::String::<32>::from_str(d.str()?).map_err(|e| minicbor::decode::Error::message("Bad String"))?;
        let class_type: heapless::String<32> =
            heapless::String::<32>::from_str(d.str()?).map_err(|e| minicbor::decode::Error::message("Bad String"))?;
        let version: heapless::String<8> = heapless::String::<8>::from_str(d.str()?).map_err(|e| minicbor::decode::Error::message("Bad String"))?;
        let code = d.bytes()?;
        Ok(EncodedFunctionClassSpecification {
            class_id,
            class_type,
            version,
            code,
        })
    }
}
