//! Static and Voyage Related Data — message type 5.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{EpfdType, Mmsi};
use crate::string::FixedStr;

/// Static and Voyage Related Data (message type 5, 424 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticVoyageData {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// AIS version indicator (0..=3).
    pub ais_version: u8,
    /// IMO number, or 0 if not available.
    pub imo_number: u32,
    /// Radio call sign.
    pub call_sign: FixedStr<7>,
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
    /// Estimated time of arrival, month (1..=12, 0 = not available).
    pub eta_month: u8,
    /// Estimated time of arrival, day (1..=31, 0 = not available).
    pub eta_day: u8,
    /// Estimated time of arrival, UTC hour (0..=23, 24 = not available).
    pub eta_hour: u8,
    /// Estimated time of arrival, UTC minute (0..=59, 60 = not available).
    pub eta_minute: u8,
    /// Maximum present static draught, in units of 0.1 meter.
    pub draught_decimeters: u8,
    /// Destination.
    pub destination: FixedStr<20>,
    /// Data terminal equipment ready flag (`true` = not available/not ready).
    pub dte_not_ready: bool,
}

impl StaticVoyageData {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let ais_version = r.read_u8(2)?;
        let imo_number = r.read_u32(30)?;
        let call_sign = r.read_sixbit_ascii(7)?;
        let name = r.read_sixbit_ascii(20)?;
        let ship_type = r.read_u8(8)?;
        let dimension_to_bow = r.read_u16(9)?;
        let dimension_to_stern = r.read_u16(9)?;
        let dimension_to_port = r.read_u8(6)?;
        let dimension_to_starboard = r.read_u8(6)?;
        let epfd_type = EpfdType::from_raw(r.read_u8(4)?);
        let eta_month = r.read_u8(4)?;
        let eta_day = r.read_u8(5)?;
        let eta_hour = r.read_u8(5)?;
        let eta_minute = r.read_u8(6)?;
        let draught_decimeters = r.read_u8(8)?;
        let destination = r.read_sixbit_ascii(20)?;
        let dte_not_ready = r.read_bool()?;
        r.skip(1)?; // spare

        Ok(Self {
            repeat_indicator,
            mmsi,
            ais_version,
            imo_number,
            call_sign,
            name,
            ship_type,
            dimension_to_bow,
            dimension_to_stern,
            dimension_to_port,
            dimension_to_starboard,
            epfd_type,
            eta_month,
            eta_day,
            eta_hour,
            eta_minute,
            draught_decimeters,
            destination,
            dte_not_ready,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(5, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(u64::from(self.ais_version), 2)?;
        w.write_bits(u64::from(self.imo_number), 30)?;
        w.write_sixbit_ascii(self.call_sign.as_str(), 7)?;
        w.write_sixbit_ascii(self.name.as_str(), 20)?;
        w.write_bits(u64::from(self.ship_type), 8)?;
        w.write_bits(u64::from(self.dimension_to_bow), 9)?;
        w.write_bits(u64::from(self.dimension_to_stern), 9)?;
        w.write_bits(u64::from(self.dimension_to_port), 6)?;
        w.write_bits(u64::from(self.dimension_to_starboard), 6)?;
        w.write_bits(u64::from(self.epfd_type.to_raw()), 4)?;
        w.write_bits(u64::from(self.eta_month), 4)?;
        w.write_bits(u64::from(self.eta_day), 5)?;
        w.write_bits(u64::from(self.eta_hour), 5)?;
        w.write_bits(u64::from(self.eta_minute), 6)?;
        w.write_bits(u64::from(self.draught_decimeters), 8)?;
        w.write_sixbit_ascii(self.destination.as_str(), 20)?;
        w.write_bool(self.dte_not_ready)?;
        w.write_bits(0, 1)?; // spare
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string::test_padded;

    fn sample() -> StaticVoyageData {
        StaticVoyageData {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(366_053_209),
            ais_version: 0,
            imo_number: 9_074_729,
            call_sign: test_padded("WDA9674"),
            name: test_padded("EXAMPLE VESSEL"),
            ship_type: 70,
            dimension_to_bow: 100,
            dimension_to_stern: 20,
            dimension_to_port: 10,
            dimension_to_starboard: 10,
            epfd_type: EpfdType::Gps,
            eta_month: 6,
            eta_day: 15,
            eta_hour: 14,
            eta_minute: 30,
            draught_decimeters: 82,
            destination: test_padded("NEW YORK"),
            dte_not_ready: false,
        }
    }

    #[test]
    fn round_trips_through_bits() {
        let original = sample();
        let mut buf = [0u8; 128];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 5);
        let decoded = StaticVoyageData::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
