use bytes::{Bytes, BytesMut};
use ensicoin_messages::message::MessageType;
use ensicoin_serializer::{Deserialize, Deserializer};
use tokio::codec::{Decoder, Encoder};

pub struct MessageCodec {
    decoding_payload: bool,
    message_type: MessageType,
    payload_size: usize,
}

impl MessageCodec {
    pub fn new() -> MessageCodec {
        MessageCodec {
            decoding_payload: false,
            message_type: MessageType::Unknown(Vec::new()),
            payload_size: 0,
        }
    }
}

impl Decoder for MessageCodec {
    type Item = (MessageType, BytesMut);
    type Error = crate::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if !self.decoding_payload && buf.len() >= 24 {
            trace!("Reading header");
            let header = buf.split_to(24);
            let mut de = Deserializer::new(header);

            let magic = u32::deserialize(&mut de).unwrap_or(0);
            if magic != crate::constants::MAGIC {
                return Err(crate::Error::InvalidMagic(magic));
            };
            let message_type = MessageType::deserialize(&mut de).unwrap();
            let payload_length = u64::deserialize(&mut de).unwrap_or(0) as usize;
            self.decoding_payload = true;
            self.message_type = message_type;
            self.payload_size = payload_length;
            trace!(
                "message: {} of size {} to read",
                self.message_type,
                self.payload_size
            );
        }
        if self.decoding_payload && buf.len() >= self.payload_size {
            trace!("Reading payload");
            self.decoding_payload = false;
            Ok(Some((
                self.message_type.clone(),
                buf.split_to(self.payload_size),
            )))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for MessageCodec {
    type Item = Bytes;
    type Error = crate::Error;

    fn encode(&mut self, raw_message: Self::Item, buf: &mut BytesMut) -> Result<(), crate::Error> {
        buf.extend_from_slice(&raw_message);
        Ok(())
    }
}
