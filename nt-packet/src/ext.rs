use std::io::{Result, Error, ErrorKind};
use bytes::Buf;

/// Trait containing functions for reading integers from `Buf`
/// Wraps existing functions, providing a safer API without panics
pub trait BufExt: Buf {
    /// Reads an unsigned byte from `self`
    fn read_u8(&mut self) -> Result<u8> {
        if self.remaining() >= 1 {
            Ok(self.get_u8())
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "self.remaining() < 1"))
        }
    }

    /// Reads an unsigned big endian short from `self`
    fn read_u16_be(&mut self) -> Result<u16> {
        if self.remaining() >= 2 {
            Ok(self.get_u16_be())
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "self.remaining() < 2"))
        }
    }

    /// Reads an unsigned big endian integer from `self`
    fn read_u32_be(&mut self) -> Result<u32> {
        if self.remaining() >= 4 {
            Ok(self.get_u32_be())
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "self.remaining() < 4"))
        }
    }

    /// Reads an unsigned big endian long from `self`
    fn read_u64_be(&mut self) -> Result<u64> {
        if self.remaining() >= 8 {
            Ok(self.get_u64_be())
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "self.remaining() < 8"))
        }
    }

    /// Reads a signed byte from `self`
    fn read_i8(&mut self) -> Result<i8> {
        if self.remaining() >= 1 {
            Ok(self.get_i8())
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "self.remaining() < 1"))
        }
    }

    /// Reads a signed big endian short from `self`
    fn read_i16_be(&mut self) -> Result<i16> {
        if self.remaining() >= 2 {
            Ok(self.get_i16_be())
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "self.remaining() < 2"))
        }
    }

    /// Reads a signed big endian integer from `self`
    fn read_i32_be(&mut self) -> Result<i32> {
        if self.remaining() >= 4 {
            Ok(self.get_i32_be())
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "self.remaining() < 4"))
        }
    }

    /// Reads a signed big endian long from `self`
    fn read_i64_be(&mut self) -> Result<i64> {
        if self.remaining() >= 8 {
            Ok(self.get_i64_be())
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "self.remaining() < 8"))
        }
    }

    /// Reads a double precision floating point number big endian from `self`
    fn read_f64_be(&mut self) -> Result<f64> {
        if self.remaining() >= 8 {
            Ok(self.get_f64_be())
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "self.remaining() < 8"))
        }
    }
}

impl<B: Buf> BufExt for B {}
