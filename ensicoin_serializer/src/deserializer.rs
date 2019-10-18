use super::types::Sha256Result;
use super::types::VarUint;
use bytes::BytesMut;

use std::net::{IpAddr, Ipv6Addr, SocketAddr};

/// Errors possible when deserializing bytes

#[derive(Debug)]
pub enum Error {
    Message(String),
    /// Typename, type size (0 being unknown), bytes read
    BufferTooShort(&'static str, usize, usize),
    InvalidString(std::string::FromUtf8Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Message(s) => write!(f, "Error in deserializing : {}", s),
            Error::BufferTooShort(t, exp, bs) => write!(
                f,
                "Not enough bytes in buffer reading {}, expected {} got {}",
                t, exp, bs
            ),
            Error::InvalidString(utf8err) => write!(f, "Invalid String: {}", utf8err),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Structure holding the data to be deserialized
pub struct Deserializer {
    buffer: BytesMut,
}

impl Deserializer {
    /// Creates a Deserializer from a bytes vector
    pub fn new(b: BytesMut) -> Deserializer {
        Deserializer { buffer: b }
    }

    pub fn extract_bytes(&mut self, length: usize) -> Result<BytesMut> {
        let buff_length = self.buffer.len();
        if length > buff_length {
            Err(Error::BufferTooShort("bytes", length, buff_length))
        } else {
            let raw = self.buffer.split_to(length);
            #[cfg(feature = "log")]
            {
                debug!(
                    "extracted {:?} as bytes, remaining in buffer: {:?}",
                    &raw.to_vec(),
                    &self.buffer.to_vec()
                );
            }
            Ok(raw)
        }
    }

    fn deserialize_u8(&mut self) -> Result<u8> {
        let length = self.buffer.len();
        if length < 1 {
            Err(Error::BufferTooShort("u8", 1, length))
        } else {
            let raw = self.buffer.split_to(1);
            #[cfg(feature = "log")]
            {
                debug!(
                    "extracted {:?} as u8, remaining in buffer: {:?}",
                    &raw.to_vec(),
                    &self.buffer.to_vec()
                );
            }
            Ok(raw[0])
        }
    }

    fn deserialize_u16(&mut self) -> Result<u16> {
        let length = self.buffer.len();
        if length < 2 {
            Err(Error::BufferTooShort("u16", 2, length))
        } else {
            let raw = self.buffer.split_to(2);
            #[cfg(feature = "log")]
            {
                debug!(
                    "extracted {:?} as u16, remaining in buffer: {:?}",
                    &raw.to_vec(),
                    &self.buffer.to_vec()
                );
            }
            Ok(((raw[0] as u16) << 8) + (raw[1] as u16))
        }
    }

    fn deserialize_u32(&mut self) -> Result<u32> {
        let length = self.buffer.len();
        if length < 4 {
            Err(Error::BufferTooShort("u32", 4, length))
        } else {
            let raw = self.buffer.split_to(4);
            #[cfg(feature = "log")]
            {
                debug!(
                    "extracted {:?} as u32, remaining in buffer: {:?}",
                    &raw.to_vec(),
                    &self.buffer.to_vec()
                );
            }
            let mut value: u32 = 0;
            for i in 1..=4 {
                value |= (raw[i - 1] as u32) << 8 * (4 - i);
            }
            Ok(value)
        }
    }

    fn deserialize_u64(&mut self) -> Result<u64> {
        let length = self.buffer.len();
        if length < 8 {
            Err(Error::BufferTooShort("u64", 8, length))
        } else {
            let raw = self.buffer.split_to(8);
            #[cfg(feature = "log")]
            {
                debug!(
                    "extracted {:?} as u64, remaining in buffer: {:?}",
                    &raw.to_vec(),
                    &self.buffer.to_vec()
                );
            }
            let mut value: u64 = 0;
            for i in 1..=8 {
                value |= (raw[i - 1] as u64) << 8 * (8 - i);
            }
            Ok(value)
        }
    }
    fn deserialize_varuint(&mut self) -> Result<VarUint> {
        let first_byte = match self.deserialize_u8() {
            Ok(n) => n,
            Err(Error::BufferTooShort(_, exp, len)) => {
                return Err(Error::BufferTooShort("VarUint", exp, len));
            }
            Err(e) => return Err(e),
        };
        let value = match first_byte {
            0xFD => match self.deserialize_u16() {
                Ok(n) => n as u64,
                Err(Error::BufferTooShort(_, exp, len)) => {
                    return Err(Error::BufferTooShort("VarUint", exp, len));
                }
                Err(e) => return Err(e),
            },
            0xFE => match self.deserialize_u32() {
                Ok(n) => n as u64,
                Err(Error::BufferTooShort(_, exp, len)) => {
                    return Err(Error::BufferTooShort("VarUint", exp, len));
                }
                Err(e) => return Err(e),
            },
            0xFF => match self.deserialize_u64() {
                Ok(n) => n as u64,
                Err(Error::BufferTooShort(_, exp, len)) => {
                    return Err(Error::BufferTooShort("VarUint", exp, len));
                }
                Err(e) => return Err(e),
            },
            _ => first_byte as u64,
        };
        Ok(VarUint { value })
    }

    fn deserialize_string(&mut self) -> Result<String> {
        let length = match self.deserialize_varuint() {
            Ok(n) => n.value as usize,
            Err(e) => {
                return Err(Error::Message(format!(
                    "Error in reading string length: {}",
                    e
                )));
            }
        };
        if self.buffer.len() < length {
            Err(Error::BufferTooShort("String", length, self.buffer.len()))
        } else {
            #[cfg(feature = "log")]
            {
                debug!(
                    "extracting {} bytes as string, remaining in buffer: {:?}",
                    length,
                    &self.buffer.to_vec()
                );
            }
            let bytes = self.buffer.split_to(length);
            match String::from_utf8(bytes.to_vec()) {
                Err(utf8err) => Err(Error::InvalidString(utf8err)),
                Ok(s) => Ok(s),
            }
        }
    }

    pub fn deserialize_vec<T: Deserialize>(&mut self) -> Result<Vec<T>> {
        let length = match self.deserialize_varuint() {
            Ok(n) => n.value as usize,
            Err(e) => {
                return Err(Error::Message(format!(
                    "Error in reading vec length: {}",
                    e
                )));
            }
        };
        #[cfg(feature = "log")]
        {
            debug!(
                "extracting {} elements as vec, remaining in buffer: {:?}",
                length,
                &self.buffer.to_vec()
            );
        }
        let mut v = Vec::new();
        for i in 0..length {
            v.push(match T::deserialize(self) {
                Ok(x) => x,
                Err(e) => {
                    return Err(Error::Message(format!(
                        "Error in reading vec item {}: {}",
                        i, e
                    )))
                }
            });
        }
        Ok(v)
    }
}

/// Trait used to create an instance of a type from a Deserializer
pub trait Deserialize: Sized {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self>;
}

impl Deserialize for u8 {
    fn deserialize(de: &mut Deserializer) -> Result<u8> {
        de.deserialize_u8()
    }
}
impl Deserialize for u16 {
    fn deserialize(de: &mut Deserializer) -> Result<u16> {
        de.deserialize_u16()
    }
}
impl Deserialize for u32 {
    fn deserialize(de: &mut Deserializer) -> Result<u32> {
        de.deserialize_u32()
    }
}
impl Deserialize for u64 {
    fn deserialize(de: &mut Deserializer) -> Result<u64> {
        de.deserialize_u64()
    }
}
impl Deserialize for VarUint {
    fn deserialize(de: &mut Deserializer) -> Result<VarUint> {
        de.deserialize_varuint()
    }
}
impl Deserialize for String {
    fn deserialize(de: &mut Deserializer) -> Result<String> {
        de.deserialize_string()
    }
}
impl<T: Deserialize> Deserialize for Vec<T> {
    fn deserialize(de: &mut Deserializer) -> Result<Vec<T>> {
        de.deserialize_vec()
    }
}

impl Deserialize for Sha256Result {
    fn deserialize(de: &mut Deserializer) -> Result<Sha256Result> {
        Ok(Sha256Result::clone_from_slice(&de.extract_bytes(32)?))
    }
}

impl Deserialize for SocketAddr {
    fn deserialize(de: &mut Deserializer) -> Result<SocketAddr> {
        let high = match u64::deserialize(de) {
            Ok(x) => x,
            Err(e) => {
                return Err(Error::Message(format!(
                    "In reading SocketAddr ip high: {}",
                    e
                )));
            }
        };
        let low = match u64::deserialize(de) {
            Ok(x) => x,
            Err(e) => {
                return Err(Error::Message(format!(
                    "In reading SocketAddr ip low: {}",
                    e
                )));
            }
        };
        let addr = ((high as u128) << 64) + (low as u128);
        let port = match u16::deserialize(de) {
            Ok(x) => x,
            Err(e) => return Err(Error::Message(format!("In reading SocketAddr port: {}", e))),
        };
        Ok(SocketAddr::new(IpAddr::from(Ipv6Addr::from(addr)), port))
    }
}

#[cfg(test)]
mod tests {
    use crate::deserializer::Deserialize;
    use crate::deserializer::Deserializer;
    extern crate bytes;
    use bytes::BytesMut;

    #[test]
    fn deserialize_vec() {
        let mut v = Vec::new();
        v.push(2);
        v.push(2);
        v.push(42);
        v.push(43);
        v.push(1);
        v.push(44);
        let mut de = Deserializer {
            buffer: BytesMut::from(v),
        };
        let decoded: Vec<Vec<u8>> = Vec::deserialize(&mut de).unwrap();
        assert_eq!(vec![vec![42 as u8, 43 as u8], vec![44]], decoded);
    }

    #[test]
    fn deserialize_string() {
        let mut v = Vec::new();
        v.push(3);
        v.push(97);
        v.push(98);
        v.push(99);
        let mut de = Deserializer {
            buffer: BytesMut::from(v),
        };
        let decoded = String::deserialize(&mut de).unwrap();
        assert_eq!(String::from("abc"), decoded);
    }

    #[test]
    fn deserialize_varuint() {
        let mut v = Vec::new();
        v.push(0xFD as u8);
        v.push(42);
        v.push(43);
        let mut de = Deserializer {
            buffer: BytesMut::from(v),
        };
        let decoded = de.deserialize_varuint().unwrap();
        assert_eq!(10795, decoded.value);
    }

    #[test]
    fn deserialize_u64() {
        let mut v = Vec::new();
        v.push(42);
        v.push(43);
        v.push(44);
        v.push(45);
        v.push(46);
        v.push(47);
        v.push(48);
        v.push(49);
        let mut de = Deserializer {
            buffer: BytesMut::from(v),
        };
        let decoded = de.deserialize_u64().unwrap();
        assert_eq!(3038570946151526449, decoded);
    }

    #[test]
    fn deserialize_u32() {
        let mut v = Vec::new();
        v.push(42);
        v.push(43);
        v.push(44);
        v.push(45);
        let mut de = Deserializer {
            buffer: BytesMut::from(v),
        };
        let decoded = de.deserialize_u32().unwrap();
        assert_eq!(707472429, decoded);
    }

    #[test]
    fn deserialize_u8() {
        let mut v = Vec::new();
        v.push(125);
        let mut de = Deserializer {
            buffer: BytesMut::from(v),
        };
        let decoded = de.deserialize_u8().unwrap();
        assert_eq!(125, decoded);
    }

    #[test]
    fn deserialize_u16() {
        let mut v = Vec::new();
        v.push(10);
        v.push(15);
        let mut de = Deserializer {
            buffer: BytesMut::from(v),
        };
        let decoded = de.deserialize_u16().unwrap();
        assert_eq!(2575, decoded);
    }
}
