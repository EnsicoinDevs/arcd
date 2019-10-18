use ensicoin_messages::resource::script::{Script, OP};
use ensicoin_serializer::Sha256Result;

use ripemd160::{Digest, Ripemd160};
use secp256k1::{Message, PublicKey, Secp256k1, Signature};

pub fn execute_script(code: Script, shash: Sha256Result) -> bool {
    let code = code.into_inner();
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut i: usize = 0;
    while i < code.len() {
        match code[i] {
            OP::False => stack.push(vec![0]),
            OP::True => stack.push(vec![1]),
            OP::Dup => {
                let end = stack.last().cloned();
                match end {
                    Some(e) => stack.push(e),
                    None => return false,
                }
            }
            OP::Equal => {
                let v1 = match stack.pop() {
                    Some(n) => n,
                    None => return false,
                };
                let v2 = match stack.pop() {
                    Some(n) => n,
                    None => return false,
                };
                stack.push(vec![(v1 == v2) as u8]);
            }
            OP::Push(n) => {
                let mut val: Vec<u8> = Vec::new();
                if i + (n as usize) >= code.len() {
                    return false;
                };
                for byte in &code[i..i + (n as usize)] {
                    match byte {
                        OP::Byte(b) => val.push(b.clone()),
                        _ => return false,
                    }
                }
                i += n as usize;
                stack.push(val);
            }
            OP::Verify => {
                let top = match stack.pop() {
                    None => return false,
                    Some(v) => v,
                };
                if top == vec![0] {
                    return false;
                }
            }
            OP::Byte(_) => return false,
            OP::Hash160 => {
                let mut hasher = Ripemd160::new();
                let top = match stack.pop() {
                    None => return false,
                    Some(x) => x,
                };
                hasher.input(top);
                let result = hasher.result();
                stack.push(Vec::from(result.as_slice()));
            }
            OP::Checksig => {
                let key = match stack.pop() {
                    None => return false,
                    Some(x) => match PublicKey::from_slice(&x) {
                        Ok(k) => k,
                        Err(_) => return false,
                    },
                };
                let sig = match stack.pop() {
                    Some(x) => match Signature::from_compact(&x) {
                        Ok(s) => s,
                        _ => return false,
                    },
                    _ => return false,
                };
                let msg = Message::from_slice(&shash).unwrap();
                let secp = Secp256k1::verification_only();
                if secp.verify(&msg, &sig, &key).is_ok() {
                    stack.push(vec![1]);
                } else {
                    stack.push(vec![0]);
                }
            }
        }
    }
    match stack.pop() {
        Some(n) => n.len() == 1 && n[0] == 1,
        _ => false,
    }
}
