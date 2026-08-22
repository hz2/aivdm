//! ITU-R M.1371 message layer: typed messages decoded from (and encoded to)
//! a bit-packed AIS payload.

pub mod common;
mod types;

pub use types::{
    Ack, Acknowledge, AidToNavigationReport, BaseStationReport, BinaryAddressedMessage,
    BinaryBroadcastMessage, LongRangeBroadcast, MultiSlotBinaryMessage, PositionReportClassA,
    PositionReportClassB, PositionReportClassBExtended, SafetyRelatedAddressed,
    SafetyRelatedBroadcast, SarAircraftPositionReport, SingleSlotBinaryMessage, StaticDataReport,
    StaticDataReportPartA, StaticDataReportPartB, StaticVoyageData,
};

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
    /// Message type 5: Static and Voyage Related Data.
    StaticVoyageData(StaticVoyageData),
    /// Message type 18: Standard Class B Position Report.
    PositionReportClassB(PositionReportClassB),
    /// Message type 19: Extended Class B Position Report.
    PositionReportClassBExtended(PositionReportClassBExtended),
    /// Message type 21: Aid-to-Navigation Report.
    AidToNavigationReport(AidToNavigationReport),
    /// Message type 24: Static Data Report (Part A or Part B).
    StaticDataReport(StaticDataReport),
    /// Message types 4 and 11: Base Station Report / UTC and Date Response.
    BaseStationReport(BaseStationReport),
    /// Message type 9: Standard SAR Aircraft Position Report.
    SarAircraftPositionReport(SarAircraftPositionReport),
    /// Message type 27: Long Range AIS Broadcast message.
    LongRangeBroadcast(LongRangeBroadcast),
    /// Message type 6: Binary Addressed Message.
    BinaryAddressedMessage(BinaryAddressedMessage),
    /// Message types 7 and 13: Binary Acknowledge / Safety Related Acknowledge.
    Acknowledge(Acknowledge),
    /// Message type 8: Binary Broadcast Message.
    BinaryBroadcastMessage(BinaryBroadcastMessage),
    /// Message type 12: Addressed Safety Related Message.
    SafetyRelatedAddressed(SafetyRelatedAddressed),
    /// Message type 14: Safety Related Broadcast Message.
    SafetyRelatedBroadcast(SafetyRelatedBroadcast),
    /// Message type 25: Single Slot Binary Message.
    SingleSlotBinaryMessage(SingleSlotBinaryMessage),
    /// Message type 26: Multiple Slot Binary Message.
    MultiSlotBinaryMessage(MultiSlotBinaryMessage),
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
            5 => Ok(Self::StaticVoyageData(StaticVoyageData::decode(r)?)),
            18 => Ok(Self::PositionReportClassB(PositionReportClassB::decode(r)?)),
            19 => Ok(Self::PositionReportClassBExtended(
                PositionReportClassBExtended::decode(r)?,
            )),
            21 => Ok(Self::AidToNavigationReport(AidToNavigationReport::decode(
                r,
            )?)),
            24 => Ok(Self::StaticDataReport(StaticDataReport::decode(r)?)),
            4 | 11 => Ok(Self::BaseStationReport(BaseStationReport::decode(
                message_type,
                r,
            )?)),
            9 => Ok(Self::SarAircraftPositionReport(
                SarAircraftPositionReport::decode(r)?,
            )),
            27 => Ok(Self::LongRangeBroadcast(LongRangeBroadcast::decode(r)?)),
            6 => Ok(Self::BinaryAddressedMessage(
                BinaryAddressedMessage::decode(r)?,
            )),
            7 | 13 => Ok(Self::Acknowledge(Acknowledge::decode(message_type, r)?)),
            8 => Ok(Self::BinaryBroadcastMessage(
                BinaryBroadcastMessage::decode(r)?,
            )),
            12 => Ok(Self::SafetyRelatedAddressed(
                SafetyRelatedAddressed::decode(r)?,
            )),
            14 => Ok(Self::SafetyRelatedBroadcast(
                SafetyRelatedBroadcast::decode(r)?,
            )),
            25 => Ok(Self::SingleSlotBinaryMessage(
                SingleSlotBinaryMessage::decode(r)?,
            )),
            26 => Ok(Self::MultiSlotBinaryMessage(
                MultiSlotBinaryMessage::decode(r)?,
            )),
            other => Err(MessageError::UnknownMessageType(other)),
        }
    }

    /// The ITU-R M.1371 message type number (1-27) this message represents.
    #[must_use]
    pub const fn message_type(&self) -> u8 {
        match self {
            Self::PositionReportClassA(m) => m.message_type,
            Self::StaticVoyageData(_) => 5,
            Self::PositionReportClassB(_) => 18,
            Self::PositionReportClassBExtended(_) => 19,
            Self::AidToNavigationReport(_) => 21,
            Self::StaticDataReport(_) => 24,
            Self::BaseStationReport(m) => m.message_type,
            Self::SarAircraftPositionReport(_) => 9,
            Self::LongRangeBroadcast(_) => 27,
            Self::BinaryAddressedMessage(_) => 6,
            Self::Acknowledge(m) => m.message_type,
            Self::BinaryBroadcastMessage(_) => 8,
            Self::SafetyRelatedAddressed(_) => 12,
            Self::SafetyRelatedBroadcast(_) => 14,
            Self::SingleSlotBinaryMessage(_) => 25,
            Self::MultiSlotBinaryMessage(_) => 26,
        }
    }

    /// Encodes this message into `w`.
    ///
    /// # Errors
    /// Returns a [`BitError`] if the output buffer runs out of room.
    pub fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        match self {
            Self::PositionReportClassA(m) => m.encode(w),
            Self::StaticVoyageData(m) => m.encode(w),
            Self::PositionReportClassB(m) => m.encode(w),
            Self::PositionReportClassBExtended(m) => m.encode(w),
            Self::AidToNavigationReport(m) => m.encode(w),
            Self::StaticDataReport(m) => m.encode(w),
            Self::BaseStationReport(m) => m.encode(w),
            Self::SarAircraftPositionReport(m) => m.encode(w),
            Self::LongRangeBroadcast(m) => m.encode(w),
            Self::BinaryAddressedMessage(m) => m.encode(w),
            Self::Acknowledge(m) => m.encode(w),
            Self::BinaryBroadcastMessage(m) => m.encode(w),
            Self::SafetyRelatedAddressed(m) => m.encode(w),
            Self::SafetyRelatedBroadcast(m) => m.encode(w),
            Self::SingleSlotBinaryMessage(m) => m.encode(w),
            Self::MultiSlotBinaryMessage(m) => m.encode(w),
        }
    }
}
