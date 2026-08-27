//! Bit-level reader over six-bit ASCII-armored AIS payload data.

use crate::error::BitError;
use crate::string::{FixedStr, armor_char_to_sixbit};

/// Reads arbitrary-width unsigned/signed integer fields, MSB-first, directly
/// out of six-bit ASCII-armored payload bytes (as found in the comma-delimited
/// payload field of an `!AIVDM`/`!AIVDO` sentence).
///
/// There is no intermediate "unpacked bitstream" buffer: fields are decoded
/// lazily, bit by bit, straight from the armored characters.
#[derive(Debug, Clone)]
pub struct BitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
    bit_len: usize,
}

impl<'a> BitReader<'a> {
    /// Builds a reader over `armored` six-bit ASCII characters, treating the
    /// trailing `fill_bits` bits of the last character as padding to ignore.
    #[must_use]
    pub const fn new(armored: &'a [u8], fill_bits: u8) -> Self {
        let total_bits = armored.len() * 6;
        let bit_len = total_bits.saturating_sub(fill_bits as usize);
        Self {
            data: armored,
            bit_pos: 0,
            bit_len,
        }
    }

    /// Number of unread bits remaining.
    #[must_use]
    pub const fn remaining_bits(&self) -> usize {
        self.bit_len.saturating_sub(self.bit_pos)
    }

    /// Reads a single bit as a `bool`.
    ///
    /// # Errors
    /// Returns [`BitError::UnexpectedEof`] if no bits remain.
    pub fn read_bool(&mut self) -> Result<bool, BitError> {
        Ok(self.read_u64(1)? != 0)
    }

    /// Reads an unsigned field of `nbits` bits (`1..=8`) into a `u8`.
    ///
    /// # Errors
    /// Returns [`BitError::OutOfRange`] if `nbits` is `0` or greater than `8`,
    /// or [`BitError::UnexpectedEof`] if not enough bits remain.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "nbits <= 8 checked above, value fits u8"
    )]
    pub fn read_u8(&mut self, nbits: u32) -> Result<u8, BitError> {
        if nbits == 0 || nbits > 8 {
            return Err(BitError::OutOfRange { field: "read_u8" });
        }
        Ok(self.read_u64(nbits)? as u8)
    }

    /// Reads an unsigned field of `nbits` bits (`1..=16`) into a `u16`.
    ///
    /// # Errors
    /// Returns [`BitError::OutOfRange`] if `nbits` is `0` or greater than `16`,
    /// or [`BitError::UnexpectedEof`] if not enough bits remain.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "nbits <= 16 checked above, value fits u16"
    )]
    pub fn read_u16(&mut self, nbits: u32) -> Result<u16, BitError> {
        if nbits == 0 || nbits > 16 {
            return Err(BitError::OutOfRange { field: "read_u16" });
        }
        Ok(self.read_u64(nbits)? as u16)
    }

    /// Reads an unsigned field of `nbits` bits (`1..=32`) into a `u32`.
    ///
    /// # Errors
    /// Returns [`BitError::OutOfRange`] if `nbits` is `0` or greater than `32`,
    /// or [`BitError::UnexpectedEof`] if not enough bits remain.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "nbits <= 32 checked above, value fits u32"
    )]
    pub fn read_u32(&mut self, nbits: u32) -> Result<u32, BitError> {
        if nbits == 0 || nbits > 32 {
            return Err(BitError::OutOfRange { field: "read_u32" });
        }
        Ok(self.read_u64(nbits)? as u32)
    }

    /// Reads an unsigned field of `nbits` bits (`1..=64`) into a `u64`.
    ///
    /// This is the workhorse that every other unsigned/signed reader delegates to.
    ///
    /// # Errors
    /// Returns [`BitError::OutOfRange`] if `nbits` is `0` or greater than `64`,
    /// or [`BitError::UnexpectedEof`] if not enough bits remain.
    pub fn read_u64(&mut self, nbits: u32) -> Result<u64, BitError> {
        if nbits == 0 || nbits > 64 {
            return Err(BitError::OutOfRange { field: "read_u64" });
        }
        let nbits = nbits as usize;
        if self.bit_pos + nbits > self.bit_len {
            return Err(BitError::UnexpectedEof);
        }
        let mut value: u64 = 0;
        for _ in 0..nbits {
            value = (value << 1) | u64::from(self.get_bit(self.bit_pos));
            self.bit_pos += 1;
        }
        Ok(value)
    }

    /// Reads a two's-complement signed field of `nbits` bits (`1..=8`) into an `i8`.
    ///
    /// # Errors
    /// Returns [`BitError::OutOfRange`] if `nbits` is `0` or greater than `8`,
    /// or [`BitError::UnexpectedEof`] if not enough bits remain.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "nbits <= 8 checked above, value fits i8"
    )]
    pub fn read_i8(&mut self, nbits: u32) -> Result<i8, BitError> {
        if nbits == 0 || nbits > 8 {
            return Err(BitError::OutOfRange { field: "read_i8" });
        }
        Ok(self.read_i64(nbits)? as i8)
    }

    /// Reads a two's-complement signed field of `nbits` bits (`1..=32`) into an `i32`.
    ///
    /// # Errors
    /// Returns [`BitError::OutOfRange`] if `nbits` is `0` or greater than `32`,
    /// or [`BitError::UnexpectedEof`] if not enough bits remain.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "nbits <= 32 checked above, value fits i32"
    )]
    pub fn read_i32(&mut self, nbits: u32) -> Result<i32, BitError> {
        if nbits == 0 || nbits > 32 {
            return Err(BitError::OutOfRange { field: "read_i32" });
        }
        Ok(self.read_i64(nbits)? as i32)
    }

    /// Reads a two's-complement signed field of `nbits` bits (`1..=64`) into an `i64`.
    ///
    /// # Errors
    /// Returns [`BitError::OutOfRange`] if `nbits` is `0` or greater than `64`,
    /// or [`BitError::UnexpectedEof`] if not enough bits remain.
    pub fn read_i64(&mut self, nbits: u32) -> Result<i64, BitError> {
        if nbits == 0 || nbits > 64 {
            return Err(BitError::OutOfRange { field: "read_i64" });
        }
        let raw = self.read_u64(nbits)?;
        Ok(sign_extend(raw, nbits))
    }

    /// Skips `nbits` bits without interpreting them (for spare/reserved fields).
    ///
    /// # Errors
    /// Returns [`BitError::UnexpectedEof`] if not enough bits remain.
    pub fn skip(&mut self, nbits: u32) -> Result<(), BitError> {
        let nbits = nbits as usize;
        if self.bit_pos + nbits > self.bit_len {
            return Err(BitError::UnexpectedEof);
        }
        self.bit_pos += nbits;
        Ok(())
    }

    /// Reads `nchars` six-bit ASCII characters (per ITU-R M.1371 Annex 8) as
    /// a fixed-capacity string, trimming trailing `@` padding and spaces.
    ///
    /// # Errors
    /// Returns [`BitError::UnexpectedEof`] if not enough bits remain, or
    /// [`BitError::OutOfRange`] if `nchars` exceeds the buffer capacity `N`.
    pub fn read_sixbit_ascii<const N: usize>(
        &mut self,
        nchars: usize,
    ) -> Result<FixedStr<N>, BitError> {
        if nchars > N {
            return Err(BitError::OutOfRange {
                field: "read_sixbit_ascii",
            });
        }
        let mut buf = [0u8; N];
        for slot in buf.iter_mut().take(nchars) {
            let sixbit = self.read_u8(6)?;
            *slot = crate::string::sixbit_to_ascii(sixbit);
        }
        Ok(FixedStr::from_raw(buf, nchars))
    }

    /// Value of bit `i` (0 = first bit of the payload), MSB-first within each
    /// six-bit armor character.
    const fn get_bit(&self, i: usize) -> u8 {
        let symbol = armor_char_to_sixbit(self.data[i / 6]);
        (symbol >> (5 - (i % 6))) & 1
    }
}

/// Sign-extends the low `nbits` bits of `raw` to a full `i64`.
const fn sign_extend(raw: u64, nbits: u32) -> i64 {
    if nbits >= 64 {
        return raw.cast_signed();
    }
    let shift = 64 - nbits;
    (raw << shift).cast_signed() >> shift
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    // BitReader::new saturates fill_bits arithmetic, and get_bit's indexing
    // is bounded by bit_len <= data.len() * 6, so no input (including an
    // out-of-range nbits or a fill_bits the caller's doc doesn't promise) can
    // ever panic here -- only the documented error returns.
    #[kani::proof]
    fn read_u64_never_panics() {
        let data: [u8; 4] = kani::any();
        let fill_bits: u8 = kani::any();
        let nbits: u32 = kani::any();
        let mut r = BitReader::new(&data, fill_bits);
        let _ = r.read_u64(nbits);
    }

    #[kani::proof]
    fn read_i64_never_panics() {
        let data: [u8; 4] = kani::any();
        let fill_bits: u8 = kani::any();
        let nbits: u32 = kani::any();
        let mut r = BitReader::new(&data, fill_bits);
        let _ = r.read_i64(nbits);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_known_unsigned_values() {
        // six armor chars '0' (=0) then 'w' (=63, all ones)
        let armored = b"0w";
        let mut r = BitReader::new(armored, 0);
        assert_eq!(r.read_u8(6).unwrap(), 0);
        assert_eq!(r.read_u8(6).unwrap(), 0b11_1111);
    }

    #[test]
    fn respects_fill_bits() {
        let armored = b"0";
        let r = BitReader::new(armored, 3);
        assert_eq!(r.remaining_bits(), 3);
    }

    #[test]
    fn eof_when_reading_past_end() {
        let armored = b"0";
        let mut r = BitReader::new(armored, 0);
        assert_eq!(r.read_u8(6).unwrap(), 0);
        assert_eq!(r.read_bool().unwrap_err(), BitError::UnexpectedEof);
    }

    #[test]
    fn sign_extend_negative_values() {
        // 4-bit field 0b1000 = -8 in two's complement
        assert_eq!(sign_extend(0b1000, 4), -8);
        // 4-bit field 0b0111 = 7
        assert_eq!(sign_extend(0b0111, 4), 7);
        // 1-bit field 1 = -1
        assert_eq!(sign_extend(0b1, 1), -1);
    }
}
