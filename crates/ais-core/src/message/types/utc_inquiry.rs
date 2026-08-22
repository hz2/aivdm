//! UTC and Date Inquiry — message type 10.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::Mmsi;

/// UTC and Date Inquiry (message type 10, 72 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcDateInquiry {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Destination station MMSI (the station being asked for a type 11 reply).
    pub destination_mmsi: Mmsi,
}

impl UtcDateInquiry {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        r.skip(2)?; // spare
        let destination_mmsi = Mmsi::from_raw(r.read_u32(30)?);
        r.skip(2)?; // spare

        Ok(Self {
            repeat_indicator,
            mmsi,
            destination_mmsi,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(10, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(0, 2)?; // spare
        w.write_bits(u64::from(self.destination_mmsi.raw()), 30)?;
        w.write_bits(0, 2)?; // spare
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_bits() {
        let original = UtcDateInquiry {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(366_053_209),
            destination_mmsi: Mmsi::from_raw(366_999_999),
        };

        let mut buf = [0u8; 16];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 10);
        let decoded = UtcDateInquiry::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
