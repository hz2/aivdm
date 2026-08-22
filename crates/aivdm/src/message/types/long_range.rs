//! Long Range AIS Broadcast message — message type 27.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{Mmsi, NavigationStatus};

/// Raw sentinel meaning "longitude not available" for the reduced-precision
/// (1/10-minute) longitude field used by message type 27.
pub const LONGITUDE_NOT_AVAILABLE_RAW: i32 = 181 * 10;
/// Raw sentinel meaning "latitude not available" for the reduced-precision
/// (1/10-minute) latitude field used by message type 27.
pub const LATITUDE_NOT_AVAILABLE_RAW: i32 = 91 * 10;

/// Long Range AIS Broadcast message (message type 27, 96 bits).
///
/// A minimal, reduced-precision position report intended for satellite
/// reception at long range; position, speed, and course all use fewer bits
/// (and coarser units) than the standard position reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LongRangeBroadcast {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Whether the reported position has better than 10m DGPS-quality accuracy.
    pub position_accuracy: bool,
    /// Whether the RAIM (Receiver Autonomous Integrity Monitoring) flag is set.
    pub raim: bool,
    /// Navigational status.
    pub navigation_status: NavigationStatus,
    /// Longitude, in 1/10-minute units (28-bit precision reduced to 18 bits).
    /// Use [`LongRangeBroadcast::longitude_degrees`] for a decoded, "not
    /// available"-aware value.
    pub longitude_raw: i32,
    /// Latitude, in 1/10-minute units (27-bit precision reduced to 17 bits).
    /// Use [`LongRangeBroadcast::latitude_degrees`] for a decoded, "not
    /// available"-aware value.
    pub latitude_raw: i32,
    /// Speed over ground, in whole knots (0..=62), or 63 if not available.
    pub sog_knots: u8,
    /// Course over ground, in whole degrees (0..=359), or 511 if not available.
    pub cog_degrees: u16,
    /// Whether the position report is more than 5 seconds old (`false` = within 5s).
    pub position_latency_high: bool,
}

impl LongRangeBroadcast {
    /// The longitude in decimal degrees, or `None` if not available.
    #[must_use]
    pub fn longitude_degrees(&self) -> Option<f64> {
        (self.longitude_raw != LONGITUDE_NOT_AVAILABLE_RAW)
            .then(|| f64::from(self.longitude_raw) / 600.0)
    }

    /// The latitude in decimal degrees, or `None` if not available.
    #[must_use]
    pub fn latitude_degrees(&self) -> Option<f64> {
        (self.latitude_raw != LATITUDE_NOT_AVAILABLE_RAW)
            .then(|| f64::from(self.latitude_raw) / 600.0)
    }

    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let position_accuracy = r.read_bool()?;
        let raim = r.read_bool()?;
        let navigation_status = NavigationStatus::from_raw(r.read_u8(4)?);
        let longitude_raw = r.read_i32(18)?;
        let latitude_raw = r.read_i32(17)?;
        let sog_knots = r.read_u8(6)?;
        let cog_degrees = r.read_u16(9)?;
        let position_latency_high = r.read_bool()?;
        r.skip(1)?; // spare

        Ok(Self {
            repeat_indicator,
            mmsi,
            position_accuracy,
            raim,
            navigation_status,
            longitude_raw,
            latitude_raw,
            sog_knots,
            cog_degrees,
            position_latency_high,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(27, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bool(self.position_accuracy)?;
        w.write_bool(self.raim)?;
        w.write_bits(u64::from(self.navigation_status.to_raw()), 4)?;
        w.write_signed(i64::from(self.longitude_raw), 18)?;
        w.write_signed(i64::from(self.latitude_raw), 17)?;
        w.write_bits(u64::from(self.sog_knots), 6)?;
        w.write_bits(u64::from(self.cog_degrees), 9)?;
        w.write_bool(self.position_latency_high)?;
        w.write_bits(0, 1)?; // spare
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> LongRangeBroadcast {
        LongRangeBroadcast {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(366_053_209),
            position_accuracy: true,
            raim: false,
            navigation_status: NavigationStatus::UnderWayUsingEngine,
            longitude_raw: -735,
            latitude_raw: 410,
            sog_knots: 12,
            cog_degrees: 270,
            position_latency_high: true,
        }
    }

    #[test]
    fn round_trips_through_bits() {
        let original = sample();
        let mut buf = [0u8; 16];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 27);
        let decoded = LongRangeBroadcast::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
