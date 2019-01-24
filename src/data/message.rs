extern crate ensicoin_serializer;
use crate::network::Connection;
use ensicoin_serializer::Serialize;

#[derive(PartialEq, Eq, Debug)]
pub enum MessageType {
    Whoami,
    WhoamiAck,
    Unknown(String),
}

pub trait Message: Serialize {
    fn message_string() -> [u8; 12];
    fn message_type() -> MessageType;
    fn send(&self, conn: &mut Connection) -> Result<(), std::io::Error> {
        let magic: u32 = 422021;
        let message_string = Self::message_string();
        let mut payload = self.serialize();
        let payload_length: u64 = payload.len() as u64;

        let mut v = Vec::new();
        v.append(&mut magic.serialize());
        v.extend_from_slice(&message_string);
        v.append(&mut payload_length.serialize());
        v.append(&mut payload);
        conn.send_bytes(Self::message_type(), v)?;
        Ok(())
    }
}
