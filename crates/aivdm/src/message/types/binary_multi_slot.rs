//! Multiple Slot Binary Message — message type 26.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{BinaryPayload, Mmsi};

/// Generous upper bound on application-data length across all addressed/
/// structured flag combinations, rounded up to whole bytes.
pub const MAX_DATA_BYTES: usize = 126;

/// Multiple Slot Binary Message (message type 26, 40..=1064 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MultiSlotBinaryMessage {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Destination station MMSI, if this message is addressed rather than broadcast.
    pub destination_mmsi: Option<Mmsi>,
    /// Designated Area Code and Functional ID, if the data is DAC/FI-structured.
    pub app_id: Option<(u16, u8)>,
    /// Application-specific binary data (excludes addressing/app-id header and
    /// the trailing communication-state fields).
    pub data: BinaryPayload<MAX_DATA_BYTES>,
    /// Communication state format selector (`false` = SOTDMA, `true` = ITDMA).
    pub communication_state_itdma: bool,
    /// Raw 19-bit communication state, undecoded.
    pub radio_status: u32,
}

impl MultiSlotBinaryMessage {
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
        // the trailing 20 bits are always the communication-state selector and state
        let data_bits = r.remaining_bits().saturating_sub(20);
        let data = BinaryPayload::decode(r, data_bits)?;
        let communication_state_itdma = r.read_bool()?;
        let radio_status = r.read_u32(19)?;

        Ok(Self {
            repeat_indicator,
            mmsi,
            destination_mmsi,
            app_id,
            data,
            communication_state_itdma,
            radio_status,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(26, 6)?;
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
        self.data.encode(w)?;
        w.write_bool(self.communication_state_itdma)?;
        w.write_bits(u64::from(self.radio_status), 19)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(addressed: bool, structured: bool) -> MultiSlotBinaryMessage {
        let mut data_buf = [0u8; MAX_DATA_BYTES];
        data_buf[0] = 0b1100_1010;
        MultiSlotBinaryMessage {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(366_053_209),
            destination_mmsi: addressed.then(|| Mmsi::from_raw(366_999_999)),
            app_id: structured.then_some((1, 21)),
            data: BinaryPayload::test_from_raw(data_buf, 16),
            communication_state_itdma: true,
            radio_status: 12345,
        }
    }

    fn round_trip(original: MultiSlotBinaryMessage) -> MultiSlotBinaryMessage {
        let mut buf = [0u8; 256];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 26);
        MultiSlotBinaryMessage::decode(&mut r).unwrap()
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
