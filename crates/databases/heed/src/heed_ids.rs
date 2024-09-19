use std::borrow::Cow;

use heed::{BoxedError, BytesDecode, BytesEncode};
use uuid::Uuid;

#[derive(Debug)]
pub struct HeedUuid(pub Uuid);

impl From<Uuid> for HeedUuid {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl<'a> BytesEncode<'a> for HeedUuid {
    type EItem = Self;

    fn bytes_encode(item: &'a Self::EItem) -> Result<Cow<'a, [u8]>, BoxedError> {
        Ok(item.0.as_bytes().into())
    }
}

impl<'a> BytesDecode<'a> for HeedUuid {
    type DItem = HeedUuid;

    fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, BoxedError> {
        Ok(HeedUuid(Uuid::from_slice(bytes)?))
    }
}

#[derive(Debug)]
pub struct HeedUuidTuple(pub (Uuid, Uuid));

impl From<(Uuid, Uuid)> for HeedUuidTuple {
    fn from(uuid: (Uuid, Uuid)) -> Self {
        Self(uuid)
    }
}

impl<'a> BytesEncode<'a> for HeedUuidTuple {
    type EItem = Self;

    fn bytes_encode(item: &'a Self::EItem) -> Result<Cow<'a, [u8]>, BoxedError> {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(item.0 .0.as_bytes());
        bytes.extend_from_slice(item.0 .1.as_bytes());
        Ok(Cow::Owned(bytes))
    }
}

impl<'a> BytesDecode<'a> for HeedUuidTuple {
    type DItem = HeedUuidTuple;

    fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, BoxedError> {
        if bytes.len() != 32 {
            return Err(BoxedError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid byte length for HeedUuidTuple",
            )));
        }
        let uuid1 = Uuid::from_slice(&bytes[..16])?;
        let uuid2 = Uuid::from_slice(&bytes[16..])?;
        Ok(HeedUuidTuple((uuid1, uuid2)))
    }
}

#[derive(Debug)]
pub struct HeedTimestampUuid(pub (u64, Uuid));

impl From<(u64, Uuid)> for HeedTimestampUuid {
    fn from(uuid: (u64, Uuid)) -> Self {
        Self(uuid)
    }
}

impl<'a> BytesEncode<'a> for HeedTimestampUuid {
    type EItem = Self;

    fn bytes_encode(item: &'a Self::EItem) -> Result<Cow<'a, [u8]>, BoxedError> {
        let mut bytes = Vec::with_capacity(24);
        bytes.extend_from_slice(&item.0 .0.to_be_bytes());
        bytes.extend_from_slice(item.0 .1.as_bytes());
        Ok(Cow::Owned(bytes))
    }
}

impl<'a> BytesDecode<'a> for HeedTimestampUuid {
    type DItem = HeedTimestampUuid;

    fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, BoxedError> {
        if bytes.len() != 24 {
            return Err(BoxedError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid byte length for HeedTimestampUuid",
            )));
        }
        let timestamp = u64::from_be_bytes(bytes[..8].try_into()?);
        let uuid = Uuid::from_slice(&bytes[8..])?;
        Ok(HeedTimestampUuid((timestamp, uuid)))
    }
}

#[derive(Debug)]
pub struct HeedMessageCreationTimeId(pub (Uuid, u64, Uuid));

impl From<(Uuid, u64, Uuid)> for HeedMessageCreationTimeId {
    fn from(uuid: (Uuid, u64, Uuid)) -> Self {
        Self(uuid)
    }
}

impl<'a> BytesEncode<'a> for HeedMessageCreationTimeId {
    type EItem = Self;

    fn bytes_encode(item: &'a Self::EItem) -> Result<Cow<'a, [u8]>, BoxedError> {
        let mut bytes = Vec::with_capacity(40);
        bytes.extend_from_slice(item.0 .0.as_bytes());
        bytes.extend_from_slice(&item.0 .1.to_be_bytes());
        bytes.extend_from_slice(item.0 .2.as_bytes());
        Ok(Cow::Owned(bytes))
    }
}

impl<'a> BytesDecode<'a> for HeedMessageCreationTimeId {
    type DItem = HeedMessageCreationTimeId;

    fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, BoxedError> {
        if bytes.len() != 40 {
            return Err(BoxedError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid byte length for HeedMessageCreationTimeId",
            )));
        }
        let u1 = Uuid::from_bytes(bytes[..16].try_into()?);
        let t = u64::from_be_bytes(bytes[16..24].try_into()?);
        let u2 = Uuid::from_bytes(bytes[24..].try_into()?);
        Ok(Self((u1, t, u2)))
    }
}
