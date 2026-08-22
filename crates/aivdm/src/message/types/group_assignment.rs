//! Group Assignment Command — message type 23.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::Mmsi;

/// Group Assignment Command (message type 23, 160 bits).
///
/// Instructs all stations of a given type within a rectangular area to
/// adopt the given reporting behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupAssignmentCommand {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI (a base station).
    pub mmsi: Mmsi,
    /// Longitude of the area's north-east corner, in 1/10-minute units.
    pub ne_longitude_raw: i32,
    /// Latitude of the area's north-east corner, in 1/10-minute units.
    pub ne_latitude_raw: i32,
    /// Longitude of the area's south-west corner, in 1/10-minute units.
    pub sw_longitude_raw: i32,
    /// Latitude of the area's south-west corner, in 1/10-minute units.
    pub sw_latitude_raw: i32,
    /// Station type the command applies to (raw code, ITU-R M.1371 Table 63).
    pub station_type: u8,
    /// Ship and cargo type filter (raw code), or 0 for "all types".
    pub ship_type: u8,
    /// Transmit/receive mode to assign.
    pub tx_rx_mode: u8,
    /// Reporting interval to assign (raw code, ITU-R M.1371 Table 64).
    pub report_interval: u8,
    /// Quiet time to assign, in minutes (0 = no quiet time commanded).
    pub quiet_time: u8,
}

impl GroupAssignmentCommand {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        r.skip(2)?; // spare
        let ne_longitude_raw = r.read_i32(18)?;
        let ne_latitude_raw = r.read_i32(17)?;
        let sw_longitude_raw = r.read_i32(18)?;
        let sw_latitude_raw = r.read_i32(17)?;
        let station_type = r.read_u8(4)?;
        let ship_type = r.read_u8(8)?;
        r.skip(22)?; // spare
        let tx_rx_mode = r.read_u8(2)?;
        let report_interval = r.read_u8(4)?;
        let quiet_time = r.read_u8(4)?;
        r.skip(6)?; // spare

        Ok(Self {
            repeat_indicator,
            mmsi,
            ne_longitude_raw,
            ne_latitude_raw,
            sw_longitude_raw,
            sw_latitude_raw,
            station_type,
            ship_type,
            tx_rx_mode,
            report_interval,
            quiet_time,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(23, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(0, 2)?; // spare
        w.write_signed(i64::from(self.ne_longitude_raw), 18)?;
        w.write_signed(i64::from(self.ne_latitude_raw), 17)?;
        w.write_signed(i64::from(self.sw_longitude_raw), 18)?;
        w.write_signed(i64::from(self.sw_latitude_raw), 17)?;
        w.write_bits(u64::from(self.station_type), 4)?;
        w.write_bits(u64::from(self.ship_type), 8)?;
        w.write_bits(0, 22)?; // spare
        w.write_bits(u64::from(self.tx_rx_mode), 2)?;
        w.write_bits(u64::from(self.report_interval), 4)?;
        w.write_bits(u64::from(self.quiet_time), 4)?;
        w.write_bits(0, 6)?; // spare
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_bits() {
        let original = GroupAssignmentCommand {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(3_669_708),
            ne_longitude_raw: 1000,
            ne_latitude_raw: 500,
            sw_longitude_raw: -1000,
            sw_latitude_raw: -500,
            station_type: 2,
            ship_type: 70,
            tx_rx_mode: 1,
            report_interval: 5,
            quiet_time: 3,
        };

        let mut buf = [0u8; 32];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 23);
        let decoded = GroupAssignmentCommand::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
