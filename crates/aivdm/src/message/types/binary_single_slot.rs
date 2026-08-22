//! Single Slot Binary Message — message type 25.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{BinaryPayload, Mmsi};

/// Maximum application-data length when neither the addressed nor structured
/// flag is set: 128 bits (168-bit max message minus the 40-bit fixed
/// header), rounded up to whole bytes.
pub const MAX_DATA_BYTES: usize = 16;

/// Single Slot Binary Message (message type 25, 40..=168 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SingleSlotBinaryMessage {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Destination station MMSI, if this message is addressed rather than broadcast.
    pub destination_mmsi: Option<Mmsi>,
    /// Designated Area Code and Functional ID, if the data is DAC/FI-structured.
    pub app_id: Option<(u16, u8)>,
    /// Application-specific binary data (excludes any addressing/app-id header).
    pub data: BinaryPayload<MAX_DATA_BYTES>,
}

impl SingleSlotBinaryMessage {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let addressed = r.read_bool()?;
        let structured = r.read_bool()?;
        let destination_mmsi = if addressed {
            Some(Mmsi::from_raw(r.read_u32(30)?))
        } else {
            None
        };
        let app_id = if structured {
            let dac = r.read_u16(10)?;
            let fi = r.read_u8(6)?;
            Some((dac, fi))
        } else {
            None
        };
        let data = BinaryPayload::decode(r, r.remaining_bits())?;

        Ok(Self {
            repeat_indicator,
            mmsi,
            destination_mmsi,
            app_id,
            data,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(25, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bool(self.destination_mmsi.is_some())?;
        w.write_bool(self.app_id.is_some())?;
        if let Some(destination_mmsi) = self.destination_mmsi {
            w.write_bits(u64::from(destination_mmsi.raw()), 30)?;
        }
        if let Some((dac, fi)) = self.app_id {
            w.write_bits(u64::from(dac), 10)?;
            w.write_bits(u64::from(fi), 6)?;
        }
        self.data.encode(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(addressed: bool, structured: bool) -> SingleSlotBinaryMessage {
        let mut data_buf = [0u8; MAX_DATA_BYTES];
        data_buf[0] = 0b1100_1010;
        SingleSlotBinaryMessage {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(366_053_209),
            destination_mmsi: addressed.then(|| Mmsi::from_raw(366_999_999)),
            app_id: structured.then_some((1, 21)),
            data: BinaryPayload::test_from_raw(data_buf, 8),
        }
    }

    fn round_trip(original: SingleSlotBinaryMessage) -> SingleSlotBinaryMessage {
        let mut buf = [0u8; 32];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 25);
        SingleSlotBinaryMessage::decode(&mut r).unwrap()
    }

    #[test]
    fn round_trips_unaddressed_unstructured() {
        let original = sample(false, false);
        assert_eq!(round_trip(original), original);
    }

    #[test]
    fn round_trips_addressed_and_structured() {
        let original = sample(true, true);
        assert_eq!(round_trip(original), original);
    }
}
