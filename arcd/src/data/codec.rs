use bytes::BufMut;
use bytes::BytesMut;
use ensicoin_messages::message::{fn_message, Message, MessageError, MessageHeader};
use tokio::codec::{Decoder, Encoder};

#[derive(Debug)]
pub enum MessageCodecError {
    IoError(tokio::io::Error),
    InvalidMessage(MessageError),
}
impl From<MessageError> for MessageCodecError {
    fn from(err: MessageError) -> Self {
        Self::InvalidMessage(err)
    }
}
impl From<tokio::io::Error> for MessageCodecError {
    fn from(err: tokio::io::Error) -> Self {
        Self::IoError(err)
    }
}

pub struct MessageCodec {
    header: Option<MessageHeader>,
}

impl MessageCodec {
    pub fn new() -> MessageCodec {
        MessageCodec { header: None }
    }
}

impl Decoder for MessageCodec {
    type Item = Message;
    type Error = MessageCodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.header.is_none() && buf.len() >= 24 {
            trace!("Reading header");
            let header = buf.split_to(24);
            let header = MessageHeader::from_bytes(crate::constants::MAGIC, header)?;

            trace!(
                "message: {} of size {} to read",
                header.message_type,
                header.payload_length
            );
            self.header = Some(header);
        }
        if let Some(header) = self.header.take() {
            if buf.len() >= header.payload_length as usize {
                trace!("Reading payload");
                let length = header.payload_length as usize;
                Ok(Some(Message::from_payload(header, buf.split_to(length))?))
            } else {
                self.header = Some(header);
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

impl Encoder for MessageCodec {
    type Item = Message;
    type Error = MessageCodecError;

    fn encode(&mut self, message: Message, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let mut vec =
            cookie_factory::gen_simple(fn_message(&message, crate::constants::MAGIC), Vec::new())
                .expect("writing message to bytes");
        buf.extend_from_slice(&mut vec);
        Ok(())
    }
}
