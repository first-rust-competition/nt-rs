use bytes::Buf;
use std::u8;

const CONTINUATION_BIT: u8 = 1 << 7;

fn low_bits_of_byte(val: u8) -> u8 {
    val & !CONTINUATION_BIT
}

pub fn read(buf: &mut dyn Buf) -> u64 {
    let mut result = 0;
    let mut shift = 0;

    loop {
        let byte = buf.get_u8();

        if shift == 63  && byte != 0x00 && byte != 0x01 {
            panic!("overflow"); // TODO: replace this with results
        }

        let low_bits = low_bits_of_byte(byte) as u64;
        result |= low_bits << shift;

        if byte & CONTINUATION_BIT == 0 {
            return result;
        }

        shift += 7;
    }
}
