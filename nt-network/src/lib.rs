mod packets;
pub mod codec;
mod ext;

pub type Result<T> = std::result::Result<T, failure::Error>;

pub use self::codec::ReceivedPacket;
pub use self::packets::*;

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NTVersion {
    V2 = 0x0200,
    V3 = 0x0300,
}


