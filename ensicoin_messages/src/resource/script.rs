use bytes::{BufMut, Bytes};
use ensicoin_serializer::{Deserialize, Deserializer, Serialize};

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum OP {
    False,
    True,
    Dup,
    Equal,
    Verify,
    Hash160,
    Checksig,
    Push(u8),
    Byte(u8),
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct Script(Vec<OP>);

impl Script {
    pub fn concat(&mut self, mut other: Script) {
        self.0.append(&mut other.0)
    }
    pub fn into_inner(self) -> Vec<OP> {
        self.0
    }
}

impl From<Vec<OP>> for Script {
    fn from(raw_script: Vec<OP>) -> Self {
        Self(raw_script)
    }
}

impl Deserialize for Script {
    fn deserialize(de: &mut Deserializer) -> ensicoin_serializer::Result<Script> {
        let mut script = Vec::new();
        let mut script_length = ensicoin_serializer::VarUint::deserialize(de)?.value as i64;
        while script_length > 0 {
            match u8::deserialize(de)? {
                0 => script.push(OP::False),
                80 => script.push(OP::True),
                n if n <= 75 => {
                    script.push(OP::Push(n));
                    for _ in 0..n {
                        script.push(OP::Byte(u8::deserialize(de)?));
                    }
                    script_length -= n as i64;
                }
                100 => script.push(OP::Dup),
                120 => script.push(OP::Equal),
                140 => script.push(OP::Verify),
                160 => script.push(OP::Hash160),
                170 => script.push(OP::Checksig),
                n => {
                    return Err(ensicoin_serializer::Error::Message(format!(
                        "Invalid opcode in context: {} (parsed: {:?})",
                        n, script
                    )))
                }
            }
            script_length -= 1;
        }
        Ok(Script(script))
    }
}

impl Serialize for Script {
    fn serialize(&self) -> Bytes {
        let script = &self.0;
        let mut bytes = bytes::BytesMut::from(
            ensicoin_serializer::VarUint {
                value: script.len() as u64,
            }
            .serialize(),
        );
        bytes.reserve(script.len());
        for op_code in script {
            let num_code = match op_code {
                OP::False => 0,
                OP::True => 80,
                OP::Push(n) | OP::Byte(n) => n.clone(),
                OP::Dup => 100,
                OP::Equal => 120,
                OP::Verify => 140,
                OP::Hash160 => 160,
                OP::Checksig => 170,
            };
            bytes.put_u8(num_code);
        }
        Bytes::from(bytes)
    }
}
