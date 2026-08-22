//! Extended Class B Equipment Position Report — message type 19.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{Cog, EpfdType, Heading, Latitude, Longitude, Mmsi, Sog, Timestamp};
use crate::string::FixedStr;

/// Extended Class B Equipment Position Report (message type 19, 312 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent single-bit wire field, not related state"
)]
pub struct PositionReportClassBExtended {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
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
    /// Vessel name.
    pub name: FixedStr<20>,
    /// Type of ship and cargo (raw code, ITU-R M.1371 Table 41).
    pub ship_type: u8,
    /// Distance from GPS antenna to the bow, in meters.
    pub dimension_to_bow: u16,
    /// Distance from GPS antenna to the stern, in meters.
    pub dimension_to_stern: u16,
    /// Distance from GPS antenna to the port side, in meters.
    pub dimension_to_port: u8,
    /// Distance from GPS antenna to the starboard side, in meters.
    pub dimension_to_starboard: u8,
    /// Electronic position fixing device type.
    pub epfd_type: EpfdType,
    /// Whether the RAIM (Receiver Autonomous Integrity Monitoring) flag is set.
    pub raim: bool,
    /// Whether the data terminal equipment is ready.
    pub dte_ready: bool,
    /// Whether the station is assigned by a message 16 or 22.
    pub assigned: bool,
}

impl PositionReportClassBExtended {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        r.skip(8)?; // regional reserved
        let sog = Sog::from_raw(r.read_u16(10)?);
        let position_accuracy = r.read_bool()?;
        let longitude = Longitude::from_raw(r.read_i32(28)?);
        let latitude = Latitude::from_raw(r.read_i32(27)?);
        let cog = Cog::from_raw(r.read_u16(12)?);
        let heading = Heading::from_raw(r.read_u16(9)?);
        let timestamp = Timestamp::from_raw(r.read_u8(6)?);
        r.skip(4)?; // regional reserved
        let name = r.read_sixbit_ascii(20)?;
        let ship_type = r.read_u8(8)?;
        let dimension_to_bow = r.read_u16(9)?;
        let dimension_to_stern = r.read_u16(9)?;
        let dimension_to_port = r.read_u8(6)?;
        let dimension_to_starboard = r.read_u8(6)?;
        let epfd_type = EpfdType::from_raw(r.read_u8(4)?);
        let raim = r.read_bool()?;
        let dte_ready = r.read_bool()?;
        let assigned = r.read_bool()?;
        r.skip(4)?; // spare

        Ok(Self {
            repeat_indicator,
            mmsi,
            sog,
            position_accuracy,
            longitude,
            latitude,
            cog,
            heading,
            timestamp,
            name,
            ship_type,
            dimension_to_bow,
            dimension_to_stern,
            dimension_to_port,
            dimension_to_starboard,
            epfd_type,
            raim,
            dte_ready,
            assigned,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(19, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(0, 8)?; // regional reserved
        w.write_bits(u64::from(self.sog.raw()), 10)?;
        w.write_bool(self.position_accuracy)?;
        w.write_signed(i64::from(self.longitude.raw()), 28)?;
        w.write_signed(i64::from(self.latitude.raw()), 27)?;
        w.write_bits(u64::from(self.cog.raw()), 12)?;
        w.write_bits(u64::from(self.heading.raw()), 9)?;
        w.write_bits(u64::from(self.timestamp.to_raw()), 6)?;
        w.write_bits(0, 4)?; // regional reserved
        w.write_sixbit_ascii(self.name.as_str(), 20)?;
        w.write_bits(u64::from(self.ship_type), 8)?;
        w.write_bits(u64::from(self.dimension_to_bow), 9)?;
        w.write_bits(u64::from(self.dimension_to_stern), 9)?;
        w.write_bits(u64::from(self.dimension_to_port), 6)?;
        w.write_bits(u64::from(self.dimension_to_starboard), 6)?;
        w.write_bits(u64::from(self.epfd_type.to_raw()), 4)?;
        w.write_bool(self.raim)?;
        w.write_bool(self.dte_ready)?;
        w.write_bool(self.assigned)?;
        w.write_bits(0, 4)?; // spare
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string::test_padded;

    fn sample() -> PositionReportClassBExtended {
        PositionReportClassBExtended {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(338_123_456),
            sog: Sog::from_raw(35),
            position_accuracy: true,
            longitude: Longitude::from_raw(-44_100_000),
            latitude: Latitude::from_raw(24_600_000),
            cog: Cog::from_raw(1234),
            heading: Heading::from_raw(88),
            timestamp: Timestamp::from_raw(42),
            name: test_padded("SAILING VESSEL"),
            ship_type: 36,
            dimension_to_bow: 12,
            dimension_to_stern: 3,
            dimension_to_port: 2,
            dimension_to_starboard: 2,
            epfd_type: EpfdType::Gps,
            raim: true,
            dte_ready: true,
            assigned: false,
        }
    }

    #[test]
    fn round_trips_through_bits() {
        let original = sample();
        let mut buf = [0u8; 64];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 19);
        let decoded = PositionReportClassBExtended::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
