/// Type representing a Unit of variable length as described in the [ensicoin
/// protocol](https://github.com/EnsicoinDevs/ensicoin/blob/master/messages.md#variable-length-integer-var_uint)
#[derive(Hash, Eq, PartialEq)]
pub struct VarUint {
    pub value: u64,
}

pub type Sha256Result = generic_array::GenericArray<u8, typenum::U32>;

pub fn hash_to_string(hash: &Sha256Result) -> String {
    hash.iter()
        .fold(String::new(), |acc, b| format!("{}{:02x}", acc, b))
}

#[cfg(test)]
mod tests {
    use crate::hash_to_string;
    use crate::Sha256Result;

    #[test]
    fn zero_hash() {
        assert_eq!(
            hash_to_string(&Sha256Result::from([0; 32])),
            String::from("0000000000000000000000000000000000000000000000000000000000000000")
        )
    }
}
