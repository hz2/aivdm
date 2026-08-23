//! ITU-R M.1371 message layer: typed messages decoded from (and encoded to)
//! a bit-packed AIS payload.

pub mod common;
mod types;

pub use types::{
    Ack, Acknowledge, AidToNavigationReport, Assignment, AssignmentModeCommand, BaseStationReport,
    BinaryAddressedMessage, BinaryBroadcastMessage, ChannelManagement, ChannelManagementTarget,
    DataLinkManagement, DgnssBroadcastMessage, GroupAssignmentCommand, Interrogation,
    LongRangeBroadcast, MessageRequest, MultiSlotBinaryMessage, PositionReportClassA,
    PositionReportClassB, PositionReportClassBExtended, SafetyRelatedAddressed,
    SafetyRelatedBroadcast, SarAircraftPositionReport, SecondStation, SingleSlotBinaryMessage,
    SlotReservation, StaticDataReport, StaticDataReportPartA, StaticDataReportPartB,
    StaticVoyageData, UtcDateInquiry,
};

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{Mmsi, NavigationStatus, Position};

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
    /// Message type 10: UTC and Date Inquiry.
    UtcDateInquiry(UtcDateInquiry),
    /// Message type 15: Interrogation.
    Interrogation(Interrogation),
    /// Message type 16: Assignment Mode Command.
    AssignmentModeCommand(AssignmentModeCommand),
    /// Message type 17: DGNSS Broadcast Binary Message.
    DgnssBroadcastMessage(DgnssBroadcastMessage),
    /// Message type 20: Data Link Management Message.
    DataLinkManagement(DataLinkManagement),
    /// Message type 22: Channel Management.
    ChannelManagement(ChannelManagement),
    /// Message type 23: Group Assignment Command.
    GroupAssignmentCommand(GroupAssignmentCommand),
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
            10 => Ok(Self::UtcDateInquiry(UtcDateInquiry::decode(r)?)),
            15 => Ok(Self::Interrogation(Interrogation::decode(r)?)),
            16 => Ok(Self::AssignmentModeCommand(AssignmentModeCommand::decode(
                r,
            )?)),
            17 => Ok(Self::DgnssBroadcastMessage(DgnssBroadcastMessage::decode(
                r,
            )?)),
            20 => Ok(Self::DataLinkManagement(DataLinkManagement::decode(r)?)),
            22 => Ok(Self::ChannelManagement(ChannelManagement::decode(r)?)),
            23 => Ok(Self::GroupAssignmentCommand(
                GroupAssignmentCommand::decode(r)?,
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
            Self::UtcDateInquiry(_) => 10,
            Self::Interrogation(_) => 15,
            Self::AssignmentModeCommand(_) => 16,
            Self::DgnssBroadcastMessage(_) => 17,
            Self::DataLinkManagement(_) => 20,
            Self::ChannelManagement(_) => 22,
            Self::GroupAssignmentCommand(_) => 23,
        }
    }

    /// How many times a repeater has relayed this message (0-3).
    ///
    /// TODO(completeness): [`StaticDataReport`]'s decoder currently discards
    /// this field (`r.skip(2)` in its wire format) instead of storing it, so
    /// this returns 0 for that variant regardless of the real value. Add a
    /// `repeat_indicator` field to `StaticDataReportPartA`/`PartB` to close
    /// this gap the same way type 21's `regional_reserved` was added.
    #[must_use]
    pub const fn repeat_indicator(&self) -> u8 {
        match self {
            Self::PositionReportClassA(m) => m.repeat_indicator,
            Self::StaticVoyageData(m) => m.repeat_indicator,
            Self::PositionReportClassB(m) => m.repeat_indicator,
            Self::PositionReportClassBExtended(m) => m.repeat_indicator,
            Self::AidToNavigationReport(m) => m.repeat_indicator,
            Self::BaseStationReport(m) => m.repeat_indicator,
            Self::SarAircraftPositionReport(m) => m.repeat_indicator,
            Self::LongRangeBroadcast(m) => m.repeat_indicator,
            Self::BinaryAddressedMessage(m) => m.repeat_indicator,
            Self::Acknowledge(m) => m.repeat_indicator,
            Self::BinaryBroadcastMessage(m) => m.repeat_indicator,
            Self::SafetyRelatedAddressed(m) => m.repeat_indicator,
            Self::SafetyRelatedBroadcast(m) => m.repeat_indicator,
            Self::SingleSlotBinaryMessage(m) => m.repeat_indicator,
            Self::MultiSlotBinaryMessage(m) => m.repeat_indicator,
            Self::UtcDateInquiry(m) => m.repeat_indicator,
            Self::Interrogation(m) => m.repeat_indicator,
            Self::AssignmentModeCommand(m) => m.repeat_indicator,
            Self::DgnssBroadcastMessage(m) => m.repeat_indicator,
            Self::DataLinkManagement(m) => m.repeat_indicator,
            Self::ChannelManagement(m) => m.repeat_indicator,
            Self::GroupAssignmentCommand(m) => m.repeat_indicator,
            Self::StaticDataReport(_) => 0,
        }
    }

    /// The source station MMSI, for the message types that carry one (which
    /// is most of them).
    ///
    /// Every message type currently defined carries an MMSI, so this match
    /// is exhaustive today; adding a future variant without an MMSI field
    /// will fail to compile here rather than silently returning a wrong answer.
    #[must_use]
    pub fn mmsi(&self) -> Mmsi {
        match self {
            Self::PositionReportClassA(m) => m.mmsi,
            Self::StaticVoyageData(m) => m.mmsi,
            Self::PositionReportClassB(m) => m.mmsi,
            Self::PositionReportClassBExtended(m) => m.mmsi,
            Self::AidToNavigationReport(m) => m.mmsi,
            Self::BaseStationReport(m) => m.mmsi,
            Self::SarAircraftPositionReport(m) => m.mmsi,
            Self::LongRangeBroadcast(m) => m.mmsi,
            Self::BinaryAddressedMessage(m) => m.mmsi,
            Self::Acknowledge(m) => m.mmsi,
            Self::BinaryBroadcastMessage(m) => m.mmsi,
            Self::SafetyRelatedAddressed(m) => m.mmsi,
            Self::SafetyRelatedBroadcast(m) => m.mmsi,
            Self::SingleSlotBinaryMessage(m) => m.mmsi,
            Self::MultiSlotBinaryMessage(m) => m.mmsi,
            Self::UtcDateInquiry(m) => m.mmsi,
            Self::Interrogation(m) => m.mmsi,
            Self::AssignmentModeCommand(m) => m.mmsi,
            Self::DgnssBroadcastMessage(m) => m.mmsi,
            Self::DataLinkManagement(m) => m.mmsi,
            Self::ChannelManagement(m) => m.mmsi,
            Self::GroupAssignmentCommand(m) => m.mmsi,
            Self::StaticDataReport(StaticDataReport::A(m)) => m.mmsi,
            Self::StaticDataReport(StaticDataReport::B(m)) => m.mmsi,
        }
    }

    /// The position, for the message types that carry one and have it
    /// marked available.
    #[must_use]
    pub fn position(&self) -> Option<Position> {
        let (latitude, longitude) = match self {
            Self::PositionReportClassA(m) => (m.latitude.as_degrees()?, m.longitude.as_degrees()?),
            Self::PositionReportClassB(m) => (m.latitude.as_degrees()?, m.longitude.as_degrees()?),
            Self::PositionReportClassBExtended(m) => {
                (m.latitude.as_degrees()?, m.longitude.as_degrees()?)
            }
            Self::AidToNavigationReport(m) => (m.latitude.as_degrees()?, m.longitude.as_degrees()?),
            Self::BaseStationReport(m) => (m.latitude.as_degrees()?, m.longitude.as_degrees()?),
            Self::SarAircraftPositionReport(m) => {
                (m.latitude.as_degrees()?, m.longitude.as_degrees()?)
            }
            Self::LongRangeBroadcast(m) => (m.latitude_degrees()?, m.longitude_degrees()?),
            _ => return None,
        };
        Some(Position {
            latitude,
            longitude,
        })
    }

    /// Speed over ground, in knots, for the message types that carry one and
    /// have it marked available.
    #[must_use]
    pub fn sog_knots(&self) -> Option<f64> {
        match self {
            Self::PositionReportClassA(m) => m.sog.knots(),
            Self::PositionReportClassB(m) => m.sog.knots(),
            Self::PositionReportClassBExtended(m) => m.sog.knots(),
            Self::SarAircraftPositionReport(m) => {
                (m.sog_knots != 1023).then(|| f64::from(m.sog_knots))
            }
            Self::LongRangeBroadcast(m) => (m.sog_knots != 63).then(|| f64::from(m.sog_knots)),
            _ => None,
        }
    }

    /// Course over ground, in degrees, for the message types that carry one
    /// and have it marked available.
    #[must_use]
    pub fn cog_degrees(&self) -> Option<f64> {
        match self {
            Self::PositionReportClassA(m) => m.cog.degrees(),
            Self::PositionReportClassB(m) => m.cog.degrees(),
            Self::PositionReportClassBExtended(m) => m.cog.degrees(),
            Self::SarAircraftPositionReport(m) => m.cog.degrees(),
            Self::LongRangeBroadcast(m) => (m.cog_degrees != 511).then(|| f64::from(m.cog_degrees)),
            _ => None,
        }
    }

    /// True heading, in whole degrees (0-359), for the message types that
    /// carry one and have it marked available.
    #[must_use]
    pub fn heading_degrees(&self) -> Option<u16> {
        match self {
            Self::PositionReportClassA(m) => m.heading.degrees(),
            Self::PositionReportClassB(m) => m.heading.degrees(),
            Self::PositionReportClassBExtended(m) => m.heading.degrees(),
            _ => None,
        }
    }

    /// Navigational status, for the message types that carry one.
    #[must_use]
    pub fn navigation_status(&self) -> Option<NavigationStatus> {
        match self {
            Self::PositionReportClassA(m) => Some(m.navigation_status),
            Self::LongRangeBroadcast(m) => Some(m.navigation_status),
            _ => None,
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
            Self::UtcDateInquiry(m) => m.encode(w),
            Self::Interrogation(m) => m.encode(w),
            Self::AssignmentModeCommand(m) => m.encode(w),
            Self::DgnssBroadcastMessage(m) => m.encode(w),
            Self::DataLinkManagement(m) => m.encode(w),
            Self::ChannelManagement(m) => m.encode(w),
            Self::GroupAssignmentCommand(m) => m.encode(w),
        }
    }
}
