use super::types::VarUint;
use std::collections::VecDeque;

pub enum Error {
    Message(String),
    BufferTooShort(usize),
    InvalidString(std::string::FromUtf8Error),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Message(s) => write!(f, "Error in deserialize : {}", s),
            Error::BufferTooShort(bs) => write!(f, "Not enough bytes in buffer : {}", bs),
            Error::InvalidString(utf8err) => utf8err.fmt(f),
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

    fn deserialize_u32(&mut self) -> Result<u32> {
        let length = self.buffer.len();
        if length < 4 {
            Err(Error::BufferTooShort(length))
        } else {
            let mut value: u32 = 0;
            for i in 1..=4 {
                value |= (self.buffer.pop_front().unwrap() as u32) << 8 * (4 - i);
            }
            Ok(value)
        }
    }

    fn deserialize_u64(&mut self) -> Result<u64> {
        let length = self.buffer.len();
        if length < 8 {
            Err(Error::BufferTooShort(length))
        } else {
            let mut value: u64 = 0;
            for i in 1..=8 {
                value |= (self.buffer.pop_front().unwrap() as u64) << 8 * (8 - i);
            }
            Ok(value)
        }
    }
    fn deserialize_varuint(&mut self) -> Result<VarUint> {
        let first_byte = self.deserialize_u8()?;
        let value = match first_byte {
            0xFD => self.deserialize_u16()? as u64,
            0xFE => self.deserialize_u32()? as u64,
            0xFF => self.deserialize_u64()?,
            _ => first_byte as u64,
        };
        Ok(VarUint { value })
    }

    fn deserialize_string(&mut self) -> Result<String> {
        let length = self.deserialize_varuint()?.value as usize;
        if self.buffer.len() < length {
            Err(Error::BufferTooShort(self.buffer.len()))
        } else {
            let mut bytes = Vec::new();
            for _ in 0..length {
                bytes.push(self.buffer.pop_front().unwrap());
            }
            match String::from_utf8(bytes) {
                Err(utf8err) => Err(Error::InvalidString(utf8err)),
                Ok(s) => Ok(s),
            }
        }
    }

    pub fn deserialize_vec<T: Deserialize>(&mut self) -> Result<Vec<T>> {
        let length = self.deserialize_varuint()?.value;
        let mut v = Vec::new();
        for _ in 0..length {
            v.push(T::deserialize(self)?);
        }
        Ok(v)
    }
}

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

#[cfg(test)]
mod tests {
    use crate::deserializer::Deserialize;
    use crate::deserializer::Deserializer;
    use std::collections::VecDeque;

    #[test]
    fn deserialize_vec() {
        let mut v = VecDeque::new();
        v.push_back(2);
        v.push_back(2);
        v.push_back(42);
        v.push_back(43);
        v.push_back(1);
        v.push_back(44);
        let mut de = Deserializer { buffer: v };
        let decoded: Vec<Vec<u8>> = Vec::deserialize(&mut de).unwrap();
        assert_eq!(vec![vec![42 as u8, 43 as u8], vec![44]], decoded);
    }

    #[test]
    fn deserialize_string() {
        let mut v = VecDeque::new();
        v.push_back(3);
        v.push_back(97);
        v.push_back(98);
        v.push_back(99);
        let mut de = Deserializer { buffer: v };
        let decoded = String::deserialize(&mut de).unwrap();
        assert_eq!(String::from("abc"), decoded);
    }

    #[test]
    fn deserialize_varuint() {
        let mut v = VecDeque::new();
        v.push_back(0xFD as u8);
        v.push_back(42);
        v.push_back(43);
        let mut de = Deserializer { buffer: v };
        let decoded = de.deserialize_varuint().unwrap();
        assert_eq!(10795, decoded.value);
    }

    #[test]
    fn deserialize_u64() {
        let mut v = VecDeque::new();
        v.push_back(42);
        v.push_back(43);
        v.push_back(44);
        v.push_back(45);
        v.push_back(46);
        v.push_back(47);
        v.push_back(48);
        v.push_back(49);
        let mut de = Deserializer { buffer: v };
        let decoded = de.deserialize_u64().unwrap();
        assert_eq!(3038570946151526449, decoded);
    }

    #[test]
    fn deserialize_u32() {
        let mut v = VecDeque::new();
        v.push_back(42);
        v.push_back(43);
        v.push_back(44);
        v.push_back(45);
        let mut de = Deserializer { buffer: v };
        let decoded = de.deserialize_u32().unwrap();
        assert_eq!(707472429, decoded);
    }

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
