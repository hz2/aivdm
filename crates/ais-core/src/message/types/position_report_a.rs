//! Position Report Class A — message types 1, 2, and 3.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{
    Cog, Heading, Latitude, Longitude, ManeuverIndicator, Mmsi, NavigationStatus, RateOfTurn, Sog,
    Timestamp,
};

/// Position Report Class A (message types 1, 2, and 3).
///
/// The three message types share an identical 168-bit wire layout; they
/// differ only in the channel access scheme the transmitting station used
/// (1 = SOTDMA/scheduled, 2 = SOTDMA/assigned, 3 = ITDMA), captured here in
/// [`PositionReportClassA::message_type`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionReportClassA {
    /// Which of message types 1, 2, or 3 this report was.
    pub message_type: u8,
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Navigational status.
    pub navigation_status: NavigationStatus,
    /// Rate of turn.
    pub rate_of_turn: RateOfTurn,
    /// Speed over ground.
    pub sog: Sog,
    /// Whether the reported position has better than 10m DGPS-quality accuracy.
    pub position_accuracy: bool,
    /// Longitude.
    pub longitude: Longitude,
    /// Latitude.
    pub latitude: Latitude,
    /// Course over ground.
    pub cog: Cog,
    /// True heading.
    pub heading: Heading,
    /// UTC second timestamp.
    pub timestamp: Timestamp,
    /// Special maneuver indicator.
    pub maneuver_indicator: ManeuverIndicator,
    /// Whether the RAIM (Receiver Autonomous Integrity Monitoring) flag is set.
    pub raim: bool,
    /// Raw 19-bit SOTDMA/ITDMA communication state, undecoded.
    pub radio_status: u32,
}

impl PositionReportClassA {
    pub(crate) fn decode(message_type: u8, r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let navigation_status = NavigationStatus::from_raw(r.read_u8(4)?);
        let rate_of_turn = RateOfTurn::from_raw(r.read_i8(8)?);
        let sog = Sog::from_raw(r.read_u16(10)?);
        let position_accuracy = r.read_bool()?;
        let longitude = Longitude::from_raw(r.read_i32(28)?);
        let latitude = Latitude::from_raw(r.read_i32(27)?);
        let cog = Cog::from_raw(r.read_u16(12)?);
        let heading = Heading::from_raw(r.read_u16(9)?);
        let timestamp = Timestamp::from_raw(r.read_u8(6)?);
        let maneuver_indicator = ManeuverIndicator::from_raw(r.read_u8(2)?);
        r.skip(3)?; // spare
        let raim = r.read_bool()?;
        let radio_status = r.read_u32(19)?;

        Ok(Self {
            message_type,
            repeat_indicator,
            mmsi,
            navigation_status,
            rate_of_turn,
            sog,
            position_accuracy,
            longitude,
            latitude,
            cog,
            heading,
            timestamp,
            maneuver_indicator,
            raim,
            radio_status,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(u64::from(self.message_type), 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(u64::from(self.navigation_status.to_raw()), 4)?;
        w.write_signed(i64::from(self.rate_of_turn.raw()), 8)?;
        w.write_bits(u64::from(self.sog.raw()), 10)?;
        w.write_bool(self.position_accuracy)?;
        w.write_signed(i64::from(self.longitude.raw()), 28)?;
        w.write_signed(i64::from(self.latitude.raw()), 27)?;
        w.write_bits(u64::from(self.cog.raw()), 12)?;
        w.write_bits(u64::from(self.heading.raw()), 9)?;
        w.write_bits(u64::from(self.timestamp.to_raw()), 6)?;
        w.write_bits(u64::from(self.maneuver_indicator.to_raw()), 2)?;
        w.write_bits(0, 3)?; // spare
        w.write_bool(self.raim)?;
        w.write_bits(u64::from(self.radio_status), 19)?;
        Ok(())
    }
}
