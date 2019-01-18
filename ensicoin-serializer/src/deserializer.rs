use super::types::VarUint;
use std::collections::VecDeque;

pub enum Error {
    Message(String),
    BufferTooShort(usize),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Message(s) => write!(f, "Error in deserialize : {}", s),
            Error::BufferTooShort(bs) => write!(f, "Not enough bytes in buffer : {}", bs),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Deserializer {
    buffer: VecDeque<u8>,
}

impl Deserializer {
    fn deserialize_u8(&mut self) -> Result<u8> {
        let length = self.buffer.len();
        if length < 1 {
            Err(Error::BufferTooShort(length))
        } else {
            Ok(self.buffer.pop_front().unwrap())
        }
    }

    fn deserialize_u16(&mut self) -> Result<u16> {
        let length = self.buffer.len();
        if length < 2 {
            Err(Error::BufferTooShort(length))
        } else {
            Ok(((self.buffer.pop_front().unwrap() as u16) << 8)
                + (self.buffer.pop_front().unwrap() as u16))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::deserializer::Deserializer;
    use std::collections::VecDeque;

    #[test]
    fn deserialize_u8() {
        let mut v = VecDeque::new();
        v.push_back(125);
        let mut de = Deserializer { buffer: v };
        let decoded = de.deserialize_u8().unwrap();
        assert_eq!(125, decoded);
    }

    #[test]
    fn deserialize_u16() {
        let mut v = VecDeque::new();
        v.push_back(10);
        v.push_back(15);
        let mut de = Deserializer { buffer: v };
        let decoded = de.deserialize_u16().unwrap();
        assert_eq!(2575, decoded);
    }
}
