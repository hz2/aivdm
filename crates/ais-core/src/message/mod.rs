//! ITU-R M.1371 message layer: typed messages decoded from (and encoded to)
//! a bit-packed AIS payload.

pub mod common;
mod types;

pub use types::PositionReportClassA;

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};

/// A decoded ITU-R M.1371 AIS message.
///
/// New variants are added as message types are implemented; matches should
/// not assume exhaustiveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AisMessage {
    /// Message types 1, 2, and 3: Position Report Class A.
    PositionReportClassA(PositionReportClassA),
}

impl AisMessage {
    /// Decodes a typed message from a bit-packed AIS payload, dispatching on
    /// the leading 6-bit message-type field.
    ///
    /// # Errors
    /// Returns [`MessageError::UnknownMessageType`] if the type field names a
    /// message type not yet implemented, or a [`MessageError::Bit`] error if
    /// the payload is truncated or otherwise malformed.
    pub fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let message_type = r.read_u8(6)?;
        match message_type {
            1..=3 => Ok(Self::PositionReportClassA(PositionReportClassA::decode(
                message_type,
                r,
            )?)),
            other => Err(MessageError::UnknownMessageType(other)),
        }
    }

    /// The ITU-R M.1371 message type number (1-27) this message represents.
    #[must_use]
    pub const fn message_type(&self) -> u8 {
        match self {
            Self::PositionReportClassA(m) => m.message_type,
        }
    }

    /// Encodes this message into `w`.
    ///
    /// # Errors
    /// Returns a [`BitError`] if the output buffer runs out of room.
    pub fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        match self {
            Self::PositionReportClassA(m) => m.encode(w),
        }
    }
}
