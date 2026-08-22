//! Addressed Safety Related Message — message type 12.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::Mmsi;
use crate::string::FixedStr;

/// Maximum safety text length: 936 bits (1008-bit max message minus the
/// 72-bit fixed header), in six-bit ASCII characters.
pub const MAX_TEXT_CHARS: usize = 156;

/// Addressed Safety Related Message (message type 12, 72..=1008 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SafetyRelatedAddressed {
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
    /// Free-text safety-related message.
    pub text: FixedStr<MAX_TEXT_CHARS>,
}

impl SafetyRelatedAddressed {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let sequence_number = r.read_u8(2)?;
        let destination_mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let retransmit = r.read_bool()?;
        r.skip(1)?; // spare
        let text_chars = (r.remaining_bits() / 6).min(MAX_TEXT_CHARS);
        let text = r.read_sixbit_ascii(text_chars)?;

        Ok(Self {
            repeat_indicator,
            mmsi,
            sequence_number,
            destination_mmsi,
            retransmit,
            text,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(12, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(u64::from(self.sequence_number), 2)?;
        w.write_bits(u64::from(self.destination_mmsi.raw()), 30)?;
        w.write_bool(self.retransmit)?;
        w.write_bits(0, 1)?; // spare
        let text = self.text.as_str();
        w.write_sixbit_ascii(text, text.chars().count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string::test_padded;

    #[test]
    fn round_trips_through_bits() {
        let original = SafetyRelatedAddressed {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(366_053_209),
            sequence_number: 1,
            destination_mmsi: Mmsi::from_raw(366_999_999),
            retransmit: false,
            text: test_padded("MAYDAY RELAY"),
        };

        let mut buf = [0u8; 256];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 12);
        let decoded = SafetyRelatedAddressed::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
