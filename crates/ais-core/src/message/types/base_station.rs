//! Base Station Report and UTC/Date Response — message types 4 and 11.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{EpfdType, Latitude, Longitude, Mmsi};

/// Base Station Report (type 4) or UTC and Date Response (type 11).
///
/// The two message types share an identical 168-bit wire layout; a base
/// station uses type 4 to broadcast its own position and the current UTC
/// time, and type 11 is the same report sent in reply to a type-10
/// UTC/date inquiry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseStationReport {
    /// Which of message types 4 or 11 this report was.
    pub message_type: u8,
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// UTC year (1..=9999), or 0 if not available.
    pub utc_year: u16,
    /// UTC month (1..=12), or 0 if not available.
    pub utc_month: u8,
    /// UTC day (1..=31), or 0 if not available.
    pub utc_day: u8,
    /// UTC hour (0..=23), or 24 if not available.
    pub utc_hour: u8,
    /// UTC minute (0..=59), or 60 if not available.
    pub utc_minute: u8,
    /// UTC second (0..=59), or 60 if not available.
    pub utc_second: u8,
    /// Whether the reported position has better than 10m DGPS-quality accuracy.
    pub position_accuracy: bool,
    /// Longitude.
    pub longitude: Longitude,
    /// Latitude.
    pub latitude: Latitude,
    /// Electronic position fixing device type.
    pub epfd_type: EpfdType,
    /// Whether the RAIM (Receiver Autonomous Integrity Monitoring) flag is set.
    pub raim: bool,
    /// Raw 19-bit SOTDMA communication state, undecoded.
    pub radio_status: u32,
}

impl BaseStationReport {
    pub(crate) fn decode(message_type: u8, r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let utc_year = r.read_u16(14)?;
        let utc_month = r.read_u8(4)?;
        let utc_day = r.read_u8(5)?;
        let utc_hour = r.read_u8(5)?;
        let utc_minute = r.read_u8(6)?;
        let utc_second = r.read_u8(6)?;
        let position_accuracy = r.read_bool()?;
        let longitude = Longitude::from_raw(r.read_i32(28)?);
        let latitude = Latitude::from_raw(r.read_i32(27)?);
        let epfd_type = EpfdType::from_raw(r.read_u8(4)?);
        r.skip(10)?; // spare
        let raim = r.read_bool()?;
        let radio_status = r.read_u32(19)?;

        Ok(Self {
            message_type,
            repeat_indicator,
            mmsi,
            utc_year,
            utc_month,
            utc_day,
            utc_hour,
            utc_minute,
            utc_second,
            position_accuracy,
            longitude,
            latitude,
            epfd_type,
            raim,
            radio_status,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(u64::from(self.message_type), 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(u64::from(self.utc_year), 14)?;
        w.write_bits(u64::from(self.utc_month), 4)?;
        w.write_bits(u64::from(self.utc_day), 5)?;
        w.write_bits(u64::from(self.utc_hour), 5)?;
        w.write_bits(u64::from(self.utc_minute), 6)?;
        w.write_bits(u64::from(self.utc_second), 6)?;
        w.write_bool(self.position_accuracy)?;
        w.write_signed(i64::from(self.longitude.raw()), 28)?;
        w.write_signed(i64::from(self.latitude.raw()), 27)?;
        w.write_bits(u64::from(self.epfd_type.to_raw()), 4)?;
        w.write_bits(0, 10)?; // spare
        w.write_bool(self.raim)?;
        w.write_bits(u64::from(self.radio_status), 19)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BaseStationReport {
        BaseStationReport {
            message_type: 4,
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(3_669_708),
            utc_year: 2024,
            utc_month: 6,
            utc_day: 15,
            utc_hour: 14,
            utc_minute: 30,
            utc_second: 45,
            position_accuracy: true,
            longitude: Longitude::from_raw(-44_100_000),
            latitude: Latitude::from_raw(24_600_000),
            epfd_type: EpfdType::Surveyed,
            raim: false,
            radio_status: 5432,
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
        assert_eq!(r.read_u8(6).unwrap(), 4);
        let decoded = BaseStationReport::decode(4, &mut r).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn round_trips_as_type_11() {
        let mut original = sample();
        original.message_type = 11;
        let mut buf = [0u8; 32];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 11);
        let decoded = BaseStationReport::decode(11, &mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
