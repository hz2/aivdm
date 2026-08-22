//! `!AIVDM`/`!AIVDO` NMEA 0183 sentence parsing.

use super::checksum;
use crate::error::NmeaError;
use crate::string::is_valid_armor_byte;

/// The VHF radio channel (or channel-number variant) a sentence was received on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// AIS channel A (161.975 MHz).
    A,
    /// AIS channel B (162.025 MHz).
    B,
    /// Some other single-byte channel designator (e.g. `'1'`/`'2'`) or unknown/empty.
    Other(u8),
}

impl Channel {
    fn from_field(s: &str) -> Self {
        match s.as_bytes() {
            [b'A'] => Self::A,
            [b'B'] => Self::B,
            [b] => Self::Other(*b),
            _ => Self::Other(0),
        }
    }
}

/// A parsed, checksum-validated `!AIVDM`/`!AIVDO` sentence.
///
/// Borrows its armored payload directly from the input line: parsing a
/// single-fragment sentence allocates nothing and copies nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence<'a> {
    /// Whether this is a simulated (`!AIVDO`, own-ship) or received (`!AIVDM`) report.
    pub is_own_ship: bool,
    /// Total number of fragments this message was split across.
    pub fragment_count: u8,
    /// This sentence's 1-based fragment number.
    pub fragment_number: u8,
    /// Sequential message ID linking fragments of a multi-part message, if present.
    pub seq_id: Option<u8>,
    /// Radio channel the sentence was received on.
    pub channel: Channel,
    /// The six-bit ASCII-armored payload bytes, borrowed from the input.
    pub payload: &'a [u8],
    /// Number of padding bits in the last armored character (`0..=5`).
    pub fill_bits: u8,
}

impl<'a> Sentence<'a> {
    /// Parses and checksum-validates a single NMEA line as an `!AIVDM`/`!AIVDO` sentence.
    ///
    /// # Errors
    /// Returns an [`NmeaError`] variant describing why the line could not be
    /// parsed: a bad checksum, an unsupported talker/formatter, a malformed
    /// or short field list, or an invalid armor character in the payload.
    pub fn parse(line: &'a str) -> Result<Self, NmeaError> {
        let line = line.trim();
        let body = line
            .strip_prefix('!')
            .or_else(|| line.strip_prefix('$'))
            .ok_or(NmeaError::MalformedSentence)?;
        let (body, after_star) = body.split_once('*').ok_or(NmeaError::MalformedSentence)?;
        // The checksum is always exactly 2 hex digits; some real-world feeds
        // (e.g. aggregators like rasHub) append extra comma-separated
        // metadata (source station, timestamp) after it, which we ignore.
        let checksum_str = after_star.get(..2).ok_or(NmeaError::MalformedSentence)?;

        let expected =
            u8::from_str_radix(checksum_str, 16).map_err(|_| NmeaError::MalformedSentence)?;
        let actual = checksum::compute(body.as_bytes());
        if expected != actual {
            return Err(NmeaError::ChecksumMismatch { expected, actual });
        }

        let mut fields = body.split(',');
        let formatter = fields.next().ok_or(NmeaError::FieldCountMismatch)?;
        let is_own_ship = if formatter.ends_with("VDO") {
            true
        } else if formatter.ends_with("VDM") {
            false
        } else {
            return Err(NmeaError::UnsupportedFormatter);
        };

        let fragment_count = parse_u8(fields.next())?;
        let fragment_number = parse_u8(fields.next())?;

        let seq_id_str = fields.next().ok_or(NmeaError::FieldCountMismatch)?;
        let seq_id = if seq_id_str.is_empty() {
            None
        } else {
            Some(parse_u8(Some(seq_id_str))?)
        };

        let channel = Channel::from_field(fields.next().ok_or(NmeaError::FieldCountMismatch)?);
        let payload_str = fields.next().ok_or(NmeaError::FieldCountMismatch)?;
        let fill_bits_str = fields.next().ok_or(NmeaError::FieldCountMismatch)?;
        if fields.next().is_some() {
            return Err(NmeaError::FieldCountMismatch);
        }

        let fill_bits: u8 = fill_bits_str
            .parse()
            .map_err(|_| NmeaError::InvalidFillBits)?;
        if fill_bits > 5 {
            return Err(NmeaError::InvalidFillBits);
        }

        let payload = payload_str.as_bytes();
        if payload.is_empty() || !payload.iter().all(|&b| is_valid_armor_byte(b)) {
            return Err(NmeaError::InvalidArmorChar);
        }

        Ok(Self {
            is_own_ship,
            fragment_count,
            fragment_number,
            seq_id,
            channel,
            payload,
            fill_bits,
        })
    }
}

fn parse_u8(field: Option<&str>) -> Result<u8, NmeaError> {
    field
        .ok_or(NmeaError::FieldCountMismatch)?
        .parse()
        .map_err(|_| NmeaError::MalformedSentence)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "!AIVDM,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*5C";

    #[test]
    fn parses_single_fragment_sentence() {
        let s = Sentence::parse(GOOD).unwrap();
        assert!(!s.is_own_ship);
        assert_eq!(s.fragment_count, 1);
        assert_eq!(s.fragment_number, 1);
        assert_eq!(s.seq_id, None);
        assert_eq!(s.channel, Channel::B);
        assert_eq!(s.fill_bits, 0);
        assert_eq!(s.payload, b"15M67FC000G?ufbE`FepT@3n00Sa");
    }

    #[test]
    fn accepts_trailing_vendor_metadata_after_checksum() {
        // Real-world aggregators (e.g. raishub) append ",<station>,<unix_ts>"
        // after the 2-digit checksum; that suffix must not affect parsing.
        let with_metadata = "!AIVDM,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*5C,raishub,1342569600";
        let s = Sentence::parse(with_metadata).unwrap();
        assert_eq!(s.payload, b"15M67FC000G?ufbE`FepT@3n00Sa");
    }

    #[test]
    fn rejects_bad_checksum() {
        let bad = "!AIVDM,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*00";
        assert!(matches!(
            Sentence::parse(bad),
            Err(NmeaError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn rejects_unsupported_formatter() {
        let bad = "!GPGGA,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*5C";
        assert!(matches!(
            Sentence::parse(bad),
            Err(NmeaError::UnsupportedFormatter | NmeaError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn rejects_missing_fields() {
        let bad = "!AIVDM,1,1*57";
        assert!(matches!(
            Sentence::parse(bad),
            Err(NmeaError::FieldCountMismatch)
        ));
    }

    #[test]
    fn recognizes_own_ship_sentences() {
        // recomputed checksum for AIVDO variant of GOOD
        let line = "!AIVDO,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*5E";
        let s = Sentence::parse(line).unwrap();
        assert!(s.is_own_ship);
    }
}
