//! Six-bit ASCII conversions and a fixed-capacity string type for decoded
//! text fields (vessel name, callsign, destination).
//!
//! Two distinct six-bit alphabets are involved:
//! - the **NMEA armor** alphabet, used to pack six-bit payload symbols into
//!   printable ASCII for the `!AIVDM`/`!AIVDO` payload field
//!   ([`armor_char_to_sixbit`]/[`sixbit_to_armor_char`]);
//! - the **ITU-R M.1371 Annex 8 six-bit ASCII** text alphabet, used *inside*
//!   the payload for string fields ([`sixbit_to_ascii`]/[`ascii_to_sixbit`]).

/// Converts one NMEA armor character back to its six-bit payload value.
///
/// Callers are expected to have validated the byte with
/// [`is_valid_armor_byte`] beforehand; invalid input does not panic but
/// produces an unspecified six-bit value.
#[must_use]
pub const fn armor_char_to_sixbit(c: u8) -> u8 {
    let v = c.wrapping_sub(48);
    if v > 40 { v.wrapping_sub(8) } else { v }
}

/// Converts a six-bit payload value (`0..=63`) to its NMEA armor character.
#[must_use]
pub const fn sixbit_to_armor_char(v: u8) -> u8 {
    let c = v + 48;
    if c > 87 { c + 8 } else { c }
}

/// Returns whether `c` is a valid NMEA armor character.
#[must_use]
pub const fn is_valid_armor_byte(c: u8) -> bool {
    matches!(c, 48..=87 | 96..=119)
}

/// Converts an ITU-R M.1371 six-bit ASCII text value (`0..=63`) to its ASCII byte.
#[must_use]
pub const fn sixbit_to_ascii(v: u8) -> u8 {
    if v < 32 { v + 64 } else { v }
}

/// Converts an ASCII byte to its ITU-R M.1371 six-bit ASCII text value.
///
/// Bytes outside the representable set (`@`-`_` and space-`?`) map to space,
/// since these fields are defined only over that alphabet.
#[must_use]
pub const fn ascii_to_sixbit(c: u8) -> u8 {
    match c {
        b'@'..=b'_' => c - 64,
        b' '..=b'?' => c,
        _ => 32,
    }
}

/// A fixed-capacity, stack-allocated string used for decoded text fields.
///
/// Trailing `@` padding and spaces (as used by the AIS six-bit text
/// convention) are trimmed by [`FixedStr::as_str`].
#[derive(Debug, Clone, Copy)]
pub struct FixedStr<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> PartialEq for FixedStr<N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<const N: usize> Eq for FixedStr<N> {}

impl<const N: usize> FixedStr<N> {
    /// Builds a `FixedStr` from a raw buffer and the number of meaningful
    /// leading bytes (including any padding still present in `buf`).
    #[must_use]
    pub const fn from_raw(buf: [u8; N], len: usize) -> Self {
        Self { buf, len }
    }

    /// The decoded text, with trailing `@` padding and spaces trimmed.
    ///
    /// The six-bit ASCII text alphabet only produces bytes in `0x20..=0x5F`,
    /// so this is always valid UTF-8.
    #[must_use]
    pub fn as_str(&self) -> &str {
        let mut end = self.len;
        while end > 0 && matches!(self.buf[end - 1], b'@' | b' ') {
            end -= 1;
        }
        core::str::from_utf8(&self.buf[..end]).unwrap_or_default()
    }
}

impl<const N: usize> core::fmt::Display for FixedStr<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Builds a full-width `FixedStr<N>` from `s`, right-padded with `@`.
///
/// Only used by message-type unit tests to build sample field values without
/// hand-counting padding characters in byte-string literals.
#[cfg(test)]
pub(crate) fn test_padded<const N: usize>(s: &str) -> FixedStr<N> {
    let mut buf = [b'@'; N];
    let bytes = s.as_bytes();
    buf[..bytes.len()].copy_from_slice(bytes);
    FixedStr::from_raw(buf, N)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armor_roundtrip_covers_full_range() {
        for v in 0u8..64 {
            let c = sixbit_to_armor_char(v);
            assert!(is_valid_armor_byte(c), "char {c} for value {v} not valid");
            assert_eq!(armor_char_to_sixbit(c), v);
        }
    }

    #[test]
    fn armor_known_values() {
        assert_eq!(sixbit_to_armor_char(0), b'0');
        assert_eq!(sixbit_to_armor_char(39), b'W');
        assert_eq!(sixbit_to_armor_char(40), b'`');
        assert_eq!(sixbit_to_armor_char(63), b'w');
    }

    #[test]
    fn text_alphabet_roundtrip_covers_full_range() {
        for v in 0u8..64 {
            let c = sixbit_to_ascii(v);
            assert_eq!(ascii_to_sixbit(c), v);
        }
    }

    #[test]
    fn fixed_str_trims_padding() {
        let mut buf = [0u8; 8];
        buf[..5].copy_from_slice(b"ABC@@");
        let s: FixedStr<8> = FixedStr::from_raw(buf, 5);
        assert_eq!(s.as_str(), "ABC");
    }
}
