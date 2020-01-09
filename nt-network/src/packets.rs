use crate::ext::*;
use crate::packets::types::{EntryType, EntryValue};
use crate::{NTVersion, Result};
use bytes::{Buf, BufMut, BytesMut};

pub mod types;

pub trait Packet: Send + Sync {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()>;
    fn deserialize(buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized;
}

#[derive(Clone, Debug)]
pub struct ClientHello {
    pub version: NTVersion,
    pub name: String,
}

impl ClientHello {
    pub fn new(version: NTVersion, name: String) -> ClientHello {
        if version == NTVersion::V2 {
            panic!("V2 is not supported");
        }

        ClientHello { version, name }
    }
}

impl Packet for ClientHello {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(0x01);
        buf.put_u16(self.version as u16);
        self.name.serialize(buf)?; // cant use the method on buf because dum
        Ok(())
    }

    fn deserialize(mut buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        let version = NTVersion::from_u16(buf.read_u16_be()?)?;
        let (name, name_bytes) = String::deserialize(buf)?;
        Ok((ClientHello { version, name }, 2 + name_bytes))
    }
}

#[derive(Debug, Clone)]
pub struct ServerHello {
    pub flags: u8,
    pub name: String,
}

impl ServerHello {
    pub fn new(flags: u8, name: String) -> ServerHello {
        ServerHello { flags, name }
    }
}

impl Packet for ServerHello {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(0x04);
        buf.put_u8(self.flags);
        self.name.serialize(buf)?;
        Ok(())
    }

    fn deserialize(mut buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        let flags = buf.read_u8()?;
        let (name, bytes) = String::deserialize(buf)?;
        Ok((ServerHello::new(flags, name), 1 + bytes))
    }
}

#[derive(Clone, Debug)]
pub struct EntryAssignment {
    pub entry_name: String,
    pub entry_type: EntryType,
    pub entry_id: u16,
    pub entry_seqnum: u16,
    pub entry_flags: u8,
    pub entry_value: EntryValue,
}

impl EntryAssignment {
    pub fn new(
        entry_name: String,
        entry_type: EntryType,
        entry_id: u16,
        entry_seqnum: u16,
        entry_flags: u8,
        entry_value: EntryValue,
    ) -> EntryAssignment {
        EntryAssignment {
            entry_name,
            entry_type,
            entry_id,
            entry_seqnum,
            entry_flags,
            entry_value,
        }
    }
}

impl Packet for EntryAssignment {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(0x10);
        self.entry_name.serialize(buf)?;
        self.entry_type.serialize(buf)?;
        buf.put_u16(self.entry_id);
        buf.put_u16(self.entry_seqnum);
        buf.put_u8(self.entry_flags);
        self.entry_type.write_value(&self.entry_value, buf)?;
        Ok(())
    }

    fn deserialize(mut buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        let (s, mut read) = String::deserialize(buf)?;
        let (ty, bytes) = EntryType::deserialize(buf)?;
        read += bytes;
        let entry_id = buf.read_u16_be()?;
        let entry_seqnum = buf.read_u16_be()?;
        let flags = buf.read_u8()?;
        let (value, bytes) = ty.read_value(buf)?;
        read += bytes;
        Ok((
            EntryAssignment::new(s, ty, entry_id, entry_seqnum, flags, value),
            5 + read,
        ))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ClientHelloComplete;

impl Packet for ClientHelloComplete {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(0x05);
        Ok(())
    }

    fn deserialize(_buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        Ok((ClientHelloComplete, 0))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ServerHelloComplete;

impl Packet for ServerHelloComplete {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(0x03);
        Ok(())
    }

    fn deserialize(_buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        Ok((ServerHelloComplete, 0))
    }
}

pub struct KeepAlive;

impl Packet for KeepAlive {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(0x00);
        Ok(())
    }

    fn deserialize(_buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        Ok((KeepAlive, 0))
    }
}

#[derive(Debug, Clone)]
pub struct ProtocolVersionUnsupported {
    pub supported_version: u16,
}

impl ProtocolVersionUnsupported {
    pub fn new(supported_version: NTVersion) -> ProtocolVersionUnsupported {
        ProtocolVersionUnsupported {
            supported_version: supported_version as _,
        }
    }
}

impl Packet for ProtocolVersionUnsupported {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(0x02);
        buf.put_u16(self.supported_version);
        Ok(())
    }

    fn deserialize(mut buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        let supported_version = buf.read_u16_be()?;
        Ok((ProtocolVersionUnsupported { supported_version }, 2))
    }
}

#[derive(Debug, Clone)]
pub struct EntryUpdate {
    pub entry_id: u16,
    pub entry_seqnum: u16,
    pub entry_type: EntryType,
    pub entry_value: EntryValue,
}

impl EntryUpdate {
    pub fn new(
        entry_id: u16,
        entry_seqnum: u16,
        entry_type: EntryType,
        entry_value: EntryValue,
    ) -> EntryUpdate {
        EntryUpdate {
            entry_id,
            entry_seqnum,
            entry_type,
            entry_value,
        }
    }
}

impl Packet for EntryUpdate {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(0x11);
        buf.put_u16(self.entry_id);
        buf.put_u16(self.entry_seqnum);
        self.entry_type.serialize(buf)?;
        self.entry_type.write_value(&self.entry_value, buf)?;
        Ok(())
    }

    fn deserialize(mut buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        let entry_id = buf.read_u16_be()?;
        let entry_seqnum = buf.read_u16_be()?;
        let (entry_type, type_bytes) = EntryType::deserialize(buf)?;
        let (entry_value, value_bytes) = entry_type.read_value(buf)?;

        Ok((
            EntryUpdate::new(entry_id, entry_seqnum, entry_type, entry_value),
            2 + 2 + type_bytes + value_bytes,
        ))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EntryFlagsUpdate {
    pub entry_id: u16,
    pub entry_flags: u8,
}

impl EntryFlagsUpdate {
    pub fn new(entry_id: u16, entry_flags: u8) -> EntryFlagsUpdate {
        EntryFlagsUpdate {
            entry_id,
            entry_flags,
        }
    }
}

impl Packet for EntryFlagsUpdate {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(0x12);
        buf.put_u16(self.entry_id);
        buf.put_u8(self.entry_flags);
        Ok(())
    }

    fn deserialize(mut buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        let entry_id = buf.read_u16_be()?;
        let entry_flags = buf.get_u8();
        Ok((
            EntryFlagsUpdate {
                entry_id,
                entry_flags,
            },
            3,
        ))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EntryDelete {
    pub entry_id: u16,
}

impl EntryDelete {
    pub fn new(entry_id: u16) -> EntryDelete {
        EntryDelete { entry_id }
    }
}

impl Packet for EntryDelete {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(0x13);
        buf.put_u16(self.entry_id);
        Ok(())
    }

    fn deserialize(mut buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        let entry_id = buf.read_u16_be()?;
        Ok((EntryDelete { entry_id }, 2))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ClearAllEntries {
    pub magic: u32,
}

impl ClearAllEntries {
    pub const fn new() -> ClearAllEntries {
        ClearAllEntries {
            magic: 0xD0_6C_B2_7A,
        }
    }

    pub fn is_valid(self) -> bool {
        self.magic == 0xD0_6C_B2_7A
    }
}

impl Packet for ClearAllEntries {
    fn serialize(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(0x14);
        buf.put_u32(self.magic);
        Ok(())
    }

    fn deserialize(mut buf: &mut dyn Buf) -> Result<(Self, usize)>
    where
        Self: Sized,
    {
        let magic = buf.read_u32_be()?;
        Ok((ClearAllEntries { magic }, 4))
    }
}
