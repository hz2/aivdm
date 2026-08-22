//! Binary Broadcast Message — message type 8.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{BinaryPayload, Mmsi};

/// Maximum application-data length: 952 bits (1008-bit max message minus the
/// 56-bit fixed header), rounded up to whole bytes.
pub const MAX_DATA_BYTES: usize = 119;

/// Binary Broadcast Message (message type 8, 56..=1008 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryBroadcastMessage {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Designated Area Code identifying the application.
    pub dac: u16,
    /// Functional ID identifying the application within its DAC.
    pub fi: u8,
    /// Application-specific binary data.
    pub data: BinaryPayload<MAX_DATA_BYTES>,
}

impl BinaryBroadcastMessage {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        r.skip(2)?; // spare
        let dac = r.read_u16(10)?;
        let fi = r.read_u8(6)?;
        let data = BinaryPayload::decode(r, r.remaining_bits())?;

        Ok(Self {
            repeat_indicator,
            mmsi,
            dac,
            fi,
            data,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(8, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(0, 2)?; // spare
        w.write_bits(u64::from(self.dac), 10)?;
        w.write_bits(u64::from(self.fi), 6)?;
        self.data.encode(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::common::BinaryPayload;

    #[test]
    fn round_trips_through_bits() {
        let mut data_buf = [0u8; MAX_DATA_BYTES];
        data_buf[0] = 0b1111_0000;
        data_buf[1] = 0b1010_0000;
        let original = BinaryBroadcastMessage {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(366_053_209),
            dac: 1,
            fi: 11,
            data: BinaryPayload::test_from_raw(data_buf, 12),
        };

        let mut buf = [0u8; 256];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 8);
        let decoded = BinaryBroadcastMessage::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
