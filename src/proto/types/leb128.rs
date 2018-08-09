use bytes::{BytesMut, BufMut, Buf};
use std::u8;

const CONTINUATION_BIT: u8 = 1 << 7;

fn low_bits_of_u64(val: u64) -> u8 {
    let byte = val & (u8::MAX as u64);
    low_bits_of_byte(byte as u8)
}

fn low_bits_of_byte(val: u8) -> u8 {
    val & !CONTINUATION_BIT
}

// Shamelessly stolen from leb128 crate, made to work with BytesMut
pub fn write(buf: &mut BytesMut, mut val: u64) -> usize {
    let mut bytes_written = 0;

    loop {
        let mut byte = low_bits_of_u64(val);
        val >>= 7;
        if val != 0 {
            byte |= CONTINUATION_BIT;
        }

        buf.put_u8(byte);

        bytes_written += 1;

        if val == 0 {
            return bytes_written;
        }
    }
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
