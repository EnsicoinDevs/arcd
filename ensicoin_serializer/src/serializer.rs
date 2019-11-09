use super::types::Sha256Result;
use super::types::VarUint;
use bytes::Bytes;
use std::net::SocketAddr;

use cookie_factory::SerializeFn;
use cookie_factory::{
    bytes::{be_u16, be_u32, be_u64, be_u8},
    combinator::{cond, string},
    multi::all,
    sequence::tuple,
};
use std::io::Write;

pub fn fn_varuint<'c, W: Write + 'c>(value: VarUint) -> impl SerializeFn<W> + 'c {
    let val = value.value;
    tuple((
        cond(val <= 252, be_u8(val as u8)),
        cond(
            253 <= val && val <= 0xFFFF,
            tuple((be_u8(0xFD), be_u16(val as u16))),
        ),
        cond(
            0x10000 <= val && val <= 0xFFFFFFFF,
            tuple((be_u8(0xFE), be_u32(val as u32))),
        ),
        cond(0x100000000 <= val, tuple((be_u8(0xFF), be_u64(val)))),
    ))
}

pub fn fn_list<'c, W: Write + 'c, G: 'c, It: 'c>(
    length: u64,
    values: It,
) -> impl SerializeFn<W> + 'c
where
    G: SerializeFn<W> + 'c,
    It: Iterator<Item = G> + Clone,
{
    tuple((fn_varuint(VarUint { value: length }), all(values)))
}

pub fn fn_str<'c, W: Write + 'c, S: AsRef<str> + 'c>(data: S) -> impl SerializeFn<W> + 'c {
    tuple((
        fn_varuint(VarUint {
            value: data.as_ref().len() as u64,
        }),
        string(data),
    ))
}

/// Trait used to serialize a type to a bytes array
pub trait Serialize {
    fn serialize(&self) -> Bytes;
}

impl Serialize for u8 {
    fn serialize(&self) -> Bytes {
        Bytes::from(vec![self.clone()])
    }
}

impl Serialize for u16 {
    fn serialize(&self) -> Bytes {
        let cp = self.clone();
        let mut v = Vec::new();
        v.push((cp >> 8) as u8);
        v.push(cp as u8);
        Bytes::from(v)
    }
}

impl Serialize for u32 {
    fn serialize(&self) -> Bytes {
        let cp = self.clone();
        let mut v = Vec::new();
        v.push((cp >> 24) as u8);
        v.push((cp >> 16) as u8);
        v.push((cp >> 8) as u8);
        v.push(cp as u8);
        Bytes::from(v)
    }
}

impl Serialize for u64 {
    fn serialize(&self) -> Bytes {
        let cp = self.clone();
        let mut v = Vec::new();
        for i in 1..=8 {
            v.push((cp >> (8 * (8 - i))) as u8);
        }
        Bytes::from(v)
    }
}

impl Serialize for VarUint {
    fn serialize(&self) -> Bytes {
        match self.value {
            0..=252 => (self.value as u8).serialize(),
            253..=0xFFFF => {
                let mut v = vec![0xFD as u8];
                v.extend_from_slice(&(self.value as u16).serialize());
                Bytes::from(v)
            }
            0x10000..=0xFFFFFFFF => {
                let mut v = vec![0xFE as u8];
                v.extend_from_slice(&(self.value as u32).serialize());
                Bytes::from(v)
            }
            0x100000000..=0xFFFFFFFFFFFFFFFF => {
                let mut v = vec![0xFF as u8];
                v.extend_from_slice(&(self.value as u64).serialize());
                Bytes::from(v)
            }
        }
    }
}

impl Serialize for String {
    fn serialize(&self) -> Bytes {
        let length = VarUint {
            value: self.len() as u64,
        };
        let mut b = length.serialize();
        b.extend_from_slice(self.as_bytes());
        b
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize(&self) -> Bytes {
        let length = VarUint {
            value: self.len() as u64,
        };
        let mut v = length.serialize();
        for x in self.iter() {
            v.extend_from_slice(&x.serialize());
        }
        v
    }
}

impl Serialize for Sha256Result {
    fn serialize(&self) -> Bytes {
        Bytes::from(self.to_vec())
    }
}

impl Serialize for SocketAddr {
    fn serialize(&self) -> Bytes {
        let mut v = Vec::new();
        match self {
            SocketAddr::V4(addr) => v.extend_from_slice(&addr.ip().to_ipv6_mapped().octets()),
            SocketAddr::V6(addr) => v.extend_from_slice(&addr.ip().octets()),
        };
        v.extend_from_slice(&self.port().serialize());
        Bytes::from(v)
    }
}

#[cfg(test)]
mod tests {
    use crate::serializer::Serialize;
    use crate::types::VarUint;

    #[test]
    fn serialize_vec() {
        let v: Vec<u8> = vec![1, 2, 3, 4, 5];
        assert_eq!(vec![5, 1, 2, 3, 4, 5], v.serialize());
    }

    #[test]
    fn serialize_string() {
        let string = "whoami".to_string();
        assert_eq!(vec![6, 119, 104, 111, 97, 109, 105], string.serialize());
    }

    #[test]
    fn serialize_varuint() {
        let var_uint = VarUint { value: 756980522 };
        assert_eq!(vec![0xFE, 45, 30, 155, 42], var_uint.serialize());
    }

    #[test]
    fn serialize_cf_varuint() {
        let var_uint = VarUint { value: 756980522 };
        let mut buf = [0u8; 5];
        let (_, pos) =
            cookie_factory::gen(crate::serializer::fn_varuint(var_uint), &mut buf[..]).unwrap();
        assert_eq!(pos, 5);
        assert_eq!(buf, [0xFE, 45, 30, 155, 42]);
    }

    #[test]
    fn serialize_uint64() {
        let x: u64 = 11420619222089223280;
        assert_eq!(vec![158, 126, 48, 172, 103, 160, 76, 112], x.serialize());
    }

    #[test]
    fn serialize_uint32() {
        assert_eq!(vec![45, 30, 155, 42], (756980522 as u32).serialize());
    }

    #[test]
    fn serialize_uint8() {
        assert_eq!(vec![152], (152 as u8).serialize());
    }
    #[test]
    fn serialize_uint16() {
        assert_eq!(vec![50, 122], (12922 as u16).serialize());
    }
}
