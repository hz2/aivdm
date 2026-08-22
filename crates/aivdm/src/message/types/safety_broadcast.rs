//! Safety Related Broadcast Message — message type 14.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::Mmsi;
use crate::string::FixedStr;

/// Maximum safety text length: 968 bits (1008-bit max message minus the
/// 40-bit fixed header), in six-bit ASCII characters.
pub const MAX_TEXT_CHARS: usize = 161;

/// Safety Related Broadcast Message (message type 14, 40..=1008 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SafetyRelatedBroadcast {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Free-text safety-related message.
    pub text: FixedStr<MAX_TEXT_CHARS>,
}

impl SafetyRelatedBroadcast {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        r.skip(2)?; // spare
        let text_chars = (r.remaining_bits() / 6).min(MAX_TEXT_CHARS);
        let text = r.read_sixbit_ascii(text_chars)?;

        Ok(Self {
            repeat_indicator,
            mmsi,
            text,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(14, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(0, 2)?; // spare
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
        let original = SafetyRelatedBroadcast {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(366_053_209),
            text: test_padded("ICE WARNING IN AREA"),
        };

        let mut buf = [0u8; 256];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 14);
        let decoded = SafetyRelatedBroadcast::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
