use super::types::VarUint;

pub trait Serialize {
    fn serialize(&self) -> Vec<u8>;
}

impl Serialize for u8 {
    fn serialize(&self) -> Vec<u8> {
        vec![self.clone()]
    }
}

impl Serialize for u16 {
    fn serialize(&self) -> Vec<u8> {
        let cp = self.clone();
        let mut v = Vec::new();
        v.push((cp >> 8) as u8);
        v.push(cp as u8);
        v
    }
}

impl Serialize for u32 {
    fn serialize(&self) -> Vec<u8> {
        let cp = self.clone();
        let mut v = Vec::new();
        v.push((cp >> 24) as u8);
        v.push((cp >> 16) as u8);
        v.push((cp >> 8) as u8);
        v.push(cp as u8);
        v
    }
}

impl Serialize for u64 {
    fn serialize(&self) -> Vec<u8> {
        let cp = self.clone();
        let mut v = Vec::new();
        for i in 1..=8 {
            v.push((cp >> (8 * (8 - i))) as u8);
        }
        v
    }
}

impl Serialize for VarUint {
    fn serialize(&self) -> Vec<u8> {
        match self.value {
            0..=252 => (self.value as u8).serialize(),
            253..=0xFFFF => {
                let mut v = vec![0xFD as u8];
                v.append(&mut (self.value as u16).serialize());
                v
            }
            0x10000..=0xFFFFFFFF => {
                let mut v = vec![0xFE as u8];
                v.append(&mut (self.value as u32).serialize());
                v
            }
            0x100000000..=0xFFFFFFFFFFFFFFFF => {
                let mut v = vec![0xFF as u8];
                v.append(&mut (self.value as u64).serialize());
                v
            }
            _ => panic!("u64 bigger than 64bits"),
        }
    }
}

impl Serialize for String {
    fn serialize(&self) -> Vec<u8> {
        let length = VarUint {
            value: self.len() as u64,
        };
        let mut v = length.serialize();
        v.append(&mut self.as_bytes().to_vec());
        v
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize(&self) -> Vec<u8> {
        let length = VarUint {
            value: self.len() as u64,
        };
        let mut v = length.serialize();
        for x in self.iter() {
            v.append(&mut x.serialize());
        }
        v
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
