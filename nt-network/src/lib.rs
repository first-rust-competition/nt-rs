pub mod codec;
mod ext;
mod packets;

use failure::bail;

pub type Result<T> = std::result::Result<T, failure::Error>;

pub use self::codec::ReceivedPacket;
pub use self::packets::*;

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NTVersion {
    V2 = 0x0200,
    V3 = 0x0300,
}

impl NTVersion {
    pub fn from_u16(v: u16) -> Result<NTVersion> {
        match v {
            0x0200 => Ok(NTVersion::V2),
            0x0300 => Ok(NTVersion::V3),
            _ => bail!("Invalid version passed in packet. {:#x}", v),
        }
    }
}
