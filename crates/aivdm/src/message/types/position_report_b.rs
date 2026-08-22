//! Standard Class B Position Report — message type 18.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{Cog, Heading, Latitude, Longitude, Mmsi, Sog, Timestamp};

/// Standard Class B Equipment Position Report (message type 18, 168 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent single-bit wire field, not related state"
)]
pub struct PositionReportClassB {
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
    /// Whether the station uses CS (Carrier Sense) rather than SOTDMA access.
    pub cs_unit: bool,
    /// Whether the station has a display capable of showing ais messages.
    pub display_flag: bool,
    /// Whether the station supports DSC.
    pub dsc_flag: bool,
    /// Whether the station operates over the whole marine band.
    pub band_flag: bool,
    /// Whether the station can accept a message 22 frequency management channel assignment.
    pub message22_flag: bool,
    /// Whether the station is assigned by a message 16 or 22.
    pub assigned: bool,
    /// Whether the RAIM (Receiver Autonomous Integrity Monitoring) flag is set.
    pub raim: bool,
    /// Raw 20-bit communication state, undecoded.
    pub radio_status: u32,
}

impl PositionReportClassB {
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
        r.skip(2)?; // regional reserved
        let cs_unit = r.read_bool()?;
        let display_flag = r.read_bool()?;
        let dsc_flag = r.read_bool()?;
        let band_flag = r.read_bool()?;
        let message22_flag = r.read_bool()?;
        let assigned = r.read_bool()?;
        let raim = r.read_bool()?;
        let radio_status = r.read_u32(20)?;

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
            cs_unit,
            display_flag,
            dsc_flag,
            band_flag,
            message22_flag,
            assigned,
            raim,
            radio_status,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(18, 6)?;
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
        w.write_bits(0, 2)?; // regional reserved
        w.write_bool(self.cs_unit)?;
        w.write_bool(self.display_flag)?;
        w.write_bool(self.dsc_flag)?;
        w.write_bool(self.band_flag)?;
        w.write_bool(self.message22_flag)?;
        w.write_bool(self.assigned)?;
        w.write_bool(self.raim)?;
        w.write_bits(u64::from(self.radio_status), 20)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PositionReportClassB {
        PositionReportClassB {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(338_123_456),
            sog: Sog::from_raw(35),
            position_accuracy: true,
            longitude: Longitude::from_raw(-44_100_000),
            latitude: Latitude::from_raw(24_600_000),
            cog: Cog::from_raw(1234),
            heading: Heading::from_raw(88),
            timestamp: Timestamp::from_raw(42),
            cs_unit: true,
            display_flag: false,
            dsc_flag: true,
            band_flag: true,
            message22_flag: false,
            assigned: false,
            raim: true,
            radio_status: 12345,
        }
    }

    #[test]
    fn round_trips_through_bits() {
        let original = sample();
        let mut buf = [0u8; 32];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 18);
        let decoded = PositionReportClassB::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
