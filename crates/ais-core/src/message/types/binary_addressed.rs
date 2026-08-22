//! Binary Addressed Message — message type 6.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{BinaryPayload, Mmsi};

/// Maximum application-data length: 920 bits (1008-bit max message minus the
/// 88-bit fixed header), rounded up to whole bytes.
pub const MAX_DATA_BYTES: usize = 115;

/// Binary Addressed Message (message type 6, 88..=1008 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryAddressedMessage {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Sequence number for this source/destination pair (0..=3).
    pub sequence_number: u8,
    /// Destination station MMSI.
    pub destination_mmsi: Mmsi,
    /// Whether this is a retransmission.
    pub retransmit: bool,
    /// Designated Area Code identifying the application.
    pub dac: u16,
    /// Functional ID identifying the application within its DAC.
    pub fi: u8,
    /// Application-specific binary data.
    pub data: BinaryPayload<MAX_DATA_BYTES>,
}

impl BinaryAddressedMessage {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let sequence_number = r.read_u8(2)?;
        let destination_mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let retransmit = r.read_bool()?;
        r.skip(1)?; // spare
        let dac = r.read_u16(10)?;
        let fi = r.read_u8(6)?;
        let data = BinaryPayload::decode(r, r.remaining_bits())?;

        Ok(Self {
            repeat_indicator,
            mmsi,
            sequence_number,
            destination_mmsi,
            retransmit,
            dac,
            fi,
            data,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(6, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(u64::from(self.sequence_number), 2)?;
        w.write_bits(u64::from(self.destination_mmsi.raw()), 30)?;
        w.write_bool(self.retransmit)?;
        w.write_bits(0, 1)?; // spare
        w.write_bits(u64::from(self.dac), 10)?;
        w.write_bits(u64::from(self.fi), 6)?;
        self.data.encode(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_bits() {
        let mut data_buf = [0u8; MAX_DATA_BYTES];
        data_buf[0] = 0b1011_0100;
        let original = BinaryAddressedMessage {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(366_053_209),
            sequence_number: 2,
            destination_mmsi: Mmsi::from_raw(366_999_999),
            retransmit: true,
            dac: 200,
            fi: 21,
            data: BinaryPayload::test_from_raw(data_buf, 8),
        };

        let mut buf = [0u8; 256];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 6);
        let decoded = BinaryAddressedMessage::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
