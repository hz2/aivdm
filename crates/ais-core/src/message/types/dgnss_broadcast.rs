//! DGNSS Broadcast Binary Message — message type 17.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{BinaryPayload, Mmsi};

/// Maximum DGNSS correction data length: 736 bits (816-bit max message minus
/// the 80-bit fixed header), rounded up to whole bytes.
pub const MAX_DATA_BYTES: usize = 92;

/// DGNSS Broadcast Binary Message (message type 17, 80..=816 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DgnssBroadcastMessage {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Longitude of the DGNSS reference station, in 1/10-minute units.
    pub longitude_raw: i32,
    /// Latitude of the DGNSS reference station, in 1/10-minute units.
    pub latitude_raw: i32,
    /// DGNSS correction data (RTCM SC-104 format).
    pub data: BinaryPayload<MAX_DATA_BYTES>,
}

impl DgnssBroadcastMessage {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        r.skip(2)?; // spare
        let longitude_raw = r.read_i32(18)?;
        let latitude_raw = r.read_i32(17)?;
        r.skip(5)?; // spare
        let data = BinaryPayload::decode(r, r.remaining_bits())?;

        Ok(Self {
            repeat_indicator,
            mmsi,
            longitude_raw,
            latitude_raw,
            data,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(17, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(0, 2)?; // spare
        w.write_signed(i64::from(self.longitude_raw), 18)?;
        w.write_signed(i64::from(self.latitude_raw), 17)?;
        w.write_bits(0, 5)?; // spare
        self.data.encode(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_bits() {
        let mut data_buf = [0u8; MAX_DATA_BYTES];
        data_buf[0] = 0b1010_1010;
        let original = DgnssBroadcastMessage {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(3_669_708),
            longitude_raw: -735,
            latitude_raw: 410,
            data: BinaryPayload::test_from_raw(data_buf, 8),
        };

        let mut buf = [0u8; 128];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 17);
        let decoded = DgnssBroadcastMessage::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
