extern crate ensicoin_serializer;
use ensicoin_serializer::{Deserialize, Deserializer, Serialize};

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

impl Serialize for OP {
    fn serialize(&self) -> Vec<u8> {
        let op_code: u8 = match self {
            OP::False => 0,
            OP::True => 80,
            OP::Push(n) | OP::Byte(n) => n.clone(),
            OP::Dup => 100,
            OP::Equal => 120,
            OP::Verify => 140,
            OP::Hash160 => 160,
            OP::Checksig => 170,
        };
        vec![op_code]
    }
}

impl Deserialize for OP {
    fn deserialize(de: &mut Deserializer) -> ensicoin_serializer::Result<OP> {
        let code = match u8::deserialize(de)? {
            0 => OP::False,
            80 => OP::True,
            n if n <= 75 => OP::Push(n),
            100 => OP::Dup,
            120 => OP::Equal,
            140 => OP::Verify,
            160 => OP::Hash160,
            170 => OP::Checksig,
            n => {
                return Err(ensicoin_serializer::Error::Message(format!(
                    "Invalid op code: {}",
                    n
                )));
            }
        };
        Ok(code)
    }
}

pub struct Script {
    code: Vec<OP>,
    stack: Vec<Vec<u8>>,
}

impl Script {
    pub fn new(code: Vec<OP>) -> Script {
        Script {
            code,
            stack: Vec::new(),
        }
    }
    fn execute(&mut self) -> bool {
        let mut i: usize = 0;
        while i < self.code.len() {
            match self.code[i] {
                OP::False => self.stack.push(vec![0]),
                OP::True => self.stack.push(vec![1]),
                OP::Dup => {
                    match self.stack.last() {
                        Some(e) => self.stack.push(e.clone()),
                        None => return false,
                    };
                }
                OP::Equal => {
                    let v1 = match self.stack.pop() {
                        Some(n) => n,
                        None => return false,
                    };
                    let v2 = match self.stack.pop() {
                        Some(n) => n,
                        None => return false,
                    };
                    self.stack.push(vec![(v1 == v2) as u8]);
                }
                OP::Push(n) => {
                    let mut val: Vec<u8> = Vec::new();
                    if i + (n as usize) >= self.code.len() {
                        return false;
                    };
                    for byte in &self.code[i..i + (n as usize)] {
                        match byte {
                            OP::Byte(b) => val.push(b.clone()),
                            _ => return false,
                        }
                    }
                    i += n as usize;
                    self.stack.push(val);
                }
                OP::Verify => {
                    let top = match self.stack.pop() {
                        None => return false,
                        Some(v) => v,
                    };
                    if top == vec![0] {
                        return false;
                    }
                }
                OP::Byte(_) => return false,
                OP::Hash160 => (),
                OP::Checksig => (),
            }
        }
        true
    }
}

impl Serialize for Script {
    fn serialize(&self) -> Vec<u8> {
        self.code.serialize()
    }
}

impl Deserialize for Script {
    fn deserialize(de: &mut Deserializer) -> ensicoin_serializer::Result<Script> {
        let length = ensicoin_serializer::VarUint::deserialize(de)?.value;
        let mut i: u64 = 0;
        let mut code = Vec::new();
        while i < length {
            match OP::deserialize(de)? {
                OP::Push(n) => {
                    code.push(OP::Push(n));
                    i += 1;
                    let mut j = 0;
                    while j < n {
                        code.push(OP::Byte(u8::deserialize(de)?));
                        j += 1;
                    }
                    i += n as u64;
                }
                op => {
                    code.push(op);
                    i += 1;
                }
            }
        }
        Ok(Script::new(code))
    }
}
