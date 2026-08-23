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

    /// The single-byte channel designator this decodes back to on the wire.
    #[must_use]
    pub const fn to_byte(self) -> u8 {
        match self {
            Self::A => b'A',
            Self::B => b'B',
            Self::Other(b) => b,
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

    /// Formats this sentence back into a wire `!AIVDM`/`!AIVDO` NMEA line
    /// (with a freshly computed checksum), writing into `buf` and returning
    /// the written slice. The result has no trailing line ending; add your
    /// transport's convention (e.g. `\r\n`) yourself.
    ///
    /// # Errors
    /// Returns [`NmeaError::BufferTooSmall`] if `buf` cannot hold the
    /// formatted line, or [`NmeaError::InvalidFillBits`] if `fill_bits` is
    /// out of the legal `0..=5` range.
    pub fn format<'b>(&self, buf: &'b mut [u8]) -> Result<&'b [u8], NmeaError> {
        if self.fill_bits > 5 {
            return Err(NmeaError::InvalidFillBits);
        }

        let mut pos = 0;
        push_byte(buf, &mut pos, b'!')?;
        let body_start = pos;
        push_bytes(
            buf,
            &mut pos,
            if self.is_own_ship { b"AIVDO" } else { b"AIVDM" },
        )?;
        push_byte(buf, &mut pos, b',')?;
        push_decimal(buf, &mut pos, self.fragment_count)?;
        push_byte(buf, &mut pos, b',')?;
        push_decimal(buf, &mut pos, self.fragment_number)?;
        push_byte(buf, &mut pos, b',')?;
        if let Some(seq_id) = self.seq_id {
            push_decimal(buf, &mut pos, seq_id)?;
        }
        push_byte(buf, &mut pos, b',')?;
        push_byte(buf, &mut pos, self.channel.to_byte())?;
        push_byte(buf, &mut pos, b',')?;
        push_bytes(buf, &mut pos, self.payload)?;
        push_byte(buf, &mut pos, b',')?;
        push_decimal(buf, &mut pos, self.fill_bits)?;
        let body_end = pos;

        let checksum = checksum::compute(&buf[body_start..body_end]);
        push_byte(buf, &mut pos, b'*')?;
        push_hex_byte(buf, &mut pos, checksum)?;

        Ok(&buf[..pos])
    }
}

fn parse_u8(field: Option<&str>) -> Result<u8, NmeaError> {
    field
        .ok_or(NmeaError::FieldCountMismatch)?
        .parse()
        .map_err(|_| NmeaError::MalformedSentence)
}

fn push_byte(buf: &mut [u8], pos: &mut usize, b: u8) -> Result<(), NmeaError> {
    let slot = buf.get_mut(*pos).ok_or(NmeaError::BufferTooSmall)?;
    *slot = b;
    *pos += 1;
    Ok(())
}

fn push_bytes(buf: &mut [u8], pos: &mut usize, bytes: &[u8]) -> Result<(), NmeaError> {
    let end = pos
        .checked_add(bytes.len())
        .ok_or(NmeaError::BufferTooSmall)?;
    let dst = buf.get_mut(*pos..end).ok_or(NmeaError::BufferTooSmall)?;
    dst.copy_from_slice(bytes);
    *pos = end;
    Ok(())
}

/// Writes `v` as decimal ASCII digits (no leading zeros, `"0"` for zero).
fn push_decimal(buf: &mut [u8], pos: &mut usize, mut v: u8) -> Result<(), NmeaError> {
    let mut digits = [0u8; 3];
    let mut n = 0;
    loop {
        digits[n] = b'0' + (v % 10);
        v /= 10;
        n += 1;
        if v == 0 {
            break;
        }
    }
    for i in (0..n).rev() {
        push_byte(buf, pos, digits[i])?;
    }
    Ok(())
}

fn push_hex_byte(buf: &mut [u8], pos: &mut usize, v: u8) -> Result<(), NmeaError> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    push_byte(buf, pos, HEX[(v >> 4) as usize])?;
    push_byte(buf, pos, HEX[(v & 0x0F) as usize])?;
    Ok(())
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

    #[test]
    fn format_round_trips_a_parsed_sentence() {
        let original = Sentence::parse(GOOD).unwrap();
        let mut buf = [0u8; 64];
        let formatted = original.format(&mut buf).unwrap();
        assert_eq!(formatted, GOOD.as_bytes());

        let reparsed = Sentence::parse(core::str::from_utf8(formatted).unwrap()).unwrap();
        assert_eq!(reparsed, original);
    }

    #[test]
    fn format_computes_a_fresh_checksum() {
        let s = Sentence {
            is_own_ship: true,
            fragment_count: 2,
            fragment_number: 1,
            seq_id: Some(7),
            channel: Channel::A,
            payload: b"15M67FC000",
            fill_bits: 0,
        };
        let mut buf = [0u8; 64];
        let formatted = s.format(&mut buf).unwrap();
        assert_eq!(formatted, b"!AIVDO,2,1,7,A,15M67FC000,0*6D");

        let reparsed = Sentence::parse(core::str::from_utf8(formatted).unwrap()).unwrap();
        assert_eq!(reparsed, s);
    }

    #[test]
    fn format_reports_buffer_too_small() {
        let s = Sentence::parse(GOOD).unwrap();
        let mut buf = [0u8; 4];
        assert_eq!(s.format(&mut buf), Err(NmeaError::BufferTooSmall));
    }

    #[test]
    fn format_rejects_invalid_fill_bits() {
        let mut s = Sentence::parse(GOOD).unwrap();
        s.fill_bits = 6;
        let mut buf = [0u8; 64];
        assert_eq!(s.format(&mut buf), Err(NmeaError::InvalidFillBits));
    }
}
