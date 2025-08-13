// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Clone, minicbor::Decode, minicbor::Encode, minicbor::CborLen)]
pub enum EventData {
    #[n(0)]
    Call(#[n(0)] DataBuffer),
    #[n(1)]
    Cast(#[n(0)] DataBuffer),
    #[n(2)]
    CallRet(#[n(0)] DataBuffer),
    #[n(3)]
    CallNoRet,
    #[n(4)]
    Err,
}

#[derive(Clone)]
pub struct DataBuffer(pub heapless::Vec<u8, 1500>);

impl<C> minicbor::Encode<C> for DataBuffer {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(self.0.as_slice())?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for DataBuffer {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        Ok(DataBuffer(
            heapless::Vec::<u8, 1500>::from_slice(d.bytes()?).map_err(|_e| minicbor::decode::Error::message("String Error"))?,
        ))
    }
}

impl<C> minicbor::CborLen<C> for DataBuffer {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        minicbor::bytes::cbor_len(self.0.as_slice(), ctx)
    }
}

#[derive(Clone, minicbor::Decode, minicbor::Encode, minicbor::CborLen)]
pub struct Event {
    #[n(0)]
    pub target: crate::instance_id::InstanceId,
    #[n(1)]
    pub source: crate::instance_id::InstanceId,
    #[n(2)]
    pub stream_id: u64,
    #[n(3)]
    pub data: EventData,
    #[n(4)]
    pub target_port: super::port::Port<32>,
    #[n(5)]
    pub span_context: SpanContext,
    #[n(6)]
    pub source_port: super::port::Port<32>,
}

#[derive(Clone, minicbor::Decode, minicbor::Encode, minicbor::CborLen)]
pub struct SpanContext {
    #[n(0)]
    pub trace_id: [u8; 16],
    #[n(1)]
    pub span_id: [u8; 8],
    #[n(2)]
    pub trace_flags: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LinkProcessingResult {
    FINAL,
    PROCESSED,
    PASSED,
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn empty_buffer_cbor_len() {
        let b = DataBuffer(heapless::Vec::new());
        // byte string tag + zero len in the same byte
        assert_eq!(minicbor::len(&b), 1);
    }

    #[test]
    fn full_buffer_cbor_len() {
        let b = DataBuffer(heapless::Vec::from_slice(&[100u8; 1500]).unwrap());
        // byte string tag + 16 bit len + payload
        assert_eq!(minicbor::len(&b), 1503);
    }
}
