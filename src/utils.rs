use ensicoin_serializer::Sha256Result;
use num_bigint::BigUint;

pub fn big_uint_to_hash(big: BigUint) -> Sha256Result {
    let mut bytes = big.to_bytes_be();
    let mut zeros = vec![0; 32 - bytes.len()];
    zeros.append(&mut bytes);
    Sha256Result::clone_from_slice(&zeros)
}
