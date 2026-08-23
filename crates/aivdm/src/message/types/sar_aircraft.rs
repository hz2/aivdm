//! Standard SAR Aircraft Position Report — message type 9.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{Cog, Latitude, Longitude, Mmsi, Timestamp};

/// Standard SAR Aircraft Position Report (message type 9, 168 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent single-bit wire field, not related state"
)]
pub struct SarAircraftPositionReport {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// GNSS altitude in meters (0..=4094), or 4095 if not available.
    pub altitude_meters: u16,
    /// Speed over ground, in whole knots (0..=1022), unlike the 0.1-knot
    /// resolution used by the other position report types. 1023 = not
    /// available, 1022 = 1022 knots or higher.
    pub sog_knots: u16,
    /// Whether the reported position has better than 10m DGPS-quality accuracy.
    pub position_accuracy: bool,
    /// Longitude.
    pub longitude: Longitude,
    /// Latitude.
    pub latitude: Latitude,
    /// Course over ground.
    pub cog: Cog,
    /// UTC second timestamp.
    pub timestamp: Timestamp,
    /// Altitude sensor type (`false` = GNSS, `true` = barometric).
    pub barometric_altitude: bool,
    /// Raw 7-bit reserved-for-regional-applications field (real-world
    /// transponders may put non-zero data here despite it being nominally
    /// reserved, as seen with type 21's `regional_reserved` byte).
    pub regional_reserved: u8,
    /// Whether the data terminal equipment is *not* ready (raw wire polarity:
    /// `true` = not available/not ready).
    pub dte_not_ready: bool,
    /// Raw 3-bit spare field, nominally unused but round-tripped rather than
    /// discarded.
    pub spare: u8,
    /// Whether the station is assigned by a message 16 or 22.
    pub assigned: bool,
    /// Whether the RAIM (Receiver Autonomous Integrity Monitoring) flag is set.
    pub raim: bool,
    /// Raw 20-bit communication state, undecoded.
    pub radio_status: u32,
}

impl SarAircraftPositionReport {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let altitude_meters = r.read_u16(12)?;
        let sog_knots = r.read_u16(10)?;
        let position_accuracy = r.read_bool()?;
        let longitude = Longitude::from_raw(r.read_i32(28)?);
        let latitude = Latitude::from_raw(r.read_i32(27)?);
        let cog = Cog::from_raw(r.read_u16(12)?);
        let timestamp = Timestamp::from_raw(r.read_u8(6)?);
        let barometric_altitude = r.read_bool()?;
        let regional_reserved = r.read_u8(7)?;
        let dte_not_ready = r.read_bool()?;
        let spare = r.read_u8(3)?;
        let assigned = r.read_bool()?;
        let raim = r.read_bool()?;
        let radio_status = r.read_u32(20)?;

        Ok(Self {
            repeat_indicator,
            mmsi,
            altitude_meters,
            sog_knots,
            position_accuracy,
            longitude,
            latitude,
            cog,
            timestamp,
            barometric_altitude,
            regional_reserved,
            dte_not_ready,
            spare,
            assigned,
            raim,
            radio_status,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(9, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(u64::from(self.altitude_meters), 12)?;
        w.write_bits(u64::from(self.sog_knots), 10)?;
        w.write_bool(self.position_accuracy)?;
        w.write_signed(i64::from(self.longitude.raw()), 28)?;
        w.write_signed(i64::from(self.latitude.raw()), 27)?;
        w.write_bits(u64::from(self.cog.raw()), 12)?;
        w.write_bits(u64::from(self.timestamp.to_raw()), 6)?;
        w.write_bool(self.barometric_altitude)?;
        w.write_bits(u64::from(self.regional_reserved), 7)?;
        w.write_bool(self.dte_not_ready)?;
        w.write_bits(u64::from(self.spare), 3)?;
        w.write_bool(self.assigned)?;
        w.write_bool(self.raim)?;
        w.write_bits(u64::from(self.radio_status), 20)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SarAircraftPositionReport {
        SarAircraftPositionReport {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(111_234_567),
            altitude_meters: 1200,
            sog_knots: 850,
            position_accuracy: false,
            longitude: Longitude::from_raw(-44_100_000),
            latitude: Latitude::from_raw(24_600_000),
            cog: Cog::from_raw(900),
            timestamp: Timestamp::from_raw(12),
            barometric_altitude: true,
            regional_reserved: 0x2A,
            dte_not_ready: true,
            spare: 0x5,
            assigned: false,
            raim: true,
            radio_status: 654_321,
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
        assert_eq!(r.read_u8(6).unwrap(), 9);
        let decoded = SarAircraftPositionReport::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
