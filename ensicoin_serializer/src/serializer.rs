use super::types::VarUint;
use std::net::SocketAddr;

use cookie_factory::SerializeFn;
use cookie_factory::{
    bytes::{be_u16, be_u32, be_u64, be_u8},
    combinator::{cond, slice, string},
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

pub fn fn_socket_addr<'c, W: Write + 'c>(socket_addr: SocketAddr) -> impl SerializeFn<W> + 'c {
    tuple((
        slice(match socket_addr {
            SocketAddr::V4(addr) => addr.ip().to_ipv6_mapped().octets(),
            SocketAddr::V6(addr) => addr.ip().octets(),
        }),
        be_u16(socket_addr.port()),
    ))
}
#[cfg(test)]
mod tests {
    use crate::types::VarUint;

    #[test]
    fn serialize_cf_varuint() {
        let var_uint = VarUint { value: 756980522 };
        let mut buf = [0u8; 5];
        let (_, pos) =
            cookie_factory::gen(crate::serializer::fn_varuint(var_uint), &mut buf[..]).unwrap();
        assert_eq!(pos, 5);
        assert_eq!(buf, [0xFE, 45, 30, 155, 42]);
    }
}
