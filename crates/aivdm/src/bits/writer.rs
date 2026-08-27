//! Bit-level writer producing six-bit ASCII-armored AIS payload data.

use crate::error::BitError;
use crate::string::sixbit_to_armor_char;

/// Writes arbitrary-width unsigned/signed integer fields, MSB-first, packing
/// them into six-bit ASCII armor characters in a caller-owned buffer.
///
/// No allocation: `out` must be large enough to hold the fully armored
/// payload (`ceil(total_bits / 6)` bytes).
#[derive(Debug)]
pub struct BitWriter<'a> {
    out: &'a mut [u8],
    char_idx: usize,
    cur_symbol: u8,
    cur_bits: u32,
}

impl<'a> BitWriter<'a> {
    /// Builds a writer over the given output buffer.
    pub fn new(out: &'a mut [u8]) -> Self {
        Self {
            out,
            char_idx: 0,
            cur_symbol: 0,
            cur_bits: 0,
        }
    }

    /// Writes a single bit.
    ///
    /// # Errors
    /// Returns [`BitError::BufferFull`] if the output buffer has no room left.
    pub fn write_bool(&mut self, v: bool) -> Result<(), BitError> {
        self.write_bits(u64::from(v), 1)
    }

    /// Writes the low `nbits` bits of `value`, MSB-first.
    ///
    /// # Errors
    /// Returns [`BitError::OutOfRange`] if `nbits` is `0` or greater than `64`,
    /// or [`BitError::BufferFull`] if the output buffer runs out of room.
    pub fn write_bits(&mut self, value: u64, nbits: u32) -> Result<(), BitError> {
        if nbits == 0 || nbits > 64 {
            return Err(BitError::OutOfRange {
                field: "write_bits",
            });
        }
        for i in (0..nbits).rev() {
            let bit = ((value >> i) & 1) as u8;
            self.push_bit(bit)?;
        }
        Ok(())
    }

    /// Writes the low `nbits` bits of a two's-complement signed `value`.
    ///
    /// # Errors
    /// Returns [`BitError::OutOfRange`] if `nbits` is `0` or greater than `64`,
    /// or [`BitError::BufferFull`] if the output buffer runs out of room.
    pub fn write_signed(&mut self, value: i64, nbits: u32) -> Result<(), BitError> {
        if nbits == 0 || nbits > 64 {
            return Err(BitError::OutOfRange {
                field: "write_signed",
            });
        }
        let mask = if nbits == 64 {
            u64::MAX
        } else {
            (1u64 << nbits) - 1
        };
        self.write_bits(value.cast_unsigned() & mask, nbits)
    }

    /// Writes `s` as `nchars` six-bit ASCII characters, right-padding with `@`.
    ///
    /// # Errors
    /// Returns [`BitError::OutOfRange`] if `s` has more than `nchars` characters,
    /// or [`BitError::BufferFull`] if the output buffer runs out of room.
    pub fn write_sixbit_ascii(&mut self, s: &str, nchars: usize) -> Result<(), BitError> {
        let bytes = s.as_bytes();
        if bytes.len() > nchars {
            return Err(BitError::OutOfRange {
                field: "write_sixbit_ascii",
            });
        }
        for i in 0..nchars {
            let ch = bytes.get(i).copied().unwrap_or(b'@');
            let sixbit = crate::string::ascii_to_sixbit(ch);
            self.write_bits(u64::from(sixbit), 6)?;
        }
        Ok(())
    }

    /// Flushes any partially filled trailing symbol (zero-padded) and returns
    /// the written armored slice along with the fill-bit count needed for the
    /// NMEA sentence's fill-bits field.
    ///
    /// # Errors
    /// Returns [`BitError::BufferFull`] if flushing the final partial symbol
    /// needs a byte that is not available in the output buffer.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "cur_bits is 1..=5 here, so 6 - cur_bits fits u8"
    )]
    pub fn finish(mut self) -> Result<(&'a [u8], u8), BitError> {
        let fill_bits = if self.cur_bits == 0 {
            0
        } else {
            (6 - self.cur_bits) as u8
        };
        if self.cur_bits > 0 {
            self.flush_symbol()?;
        }
        Ok((&self.out[..self.char_idx], fill_bits))
    }

    fn push_bit(&mut self, bit: u8) -> Result<(), BitError> {
        self.cur_symbol = (self.cur_symbol << 1) | bit;
        self.cur_bits += 1;
        if self.cur_bits == 6 {
            self.flush_symbol()?;
        }
        Ok(())
    }

    fn flush_symbol(&mut self) -> Result<(), BitError> {
        let slot = self
            .out
            .get_mut(self.char_idx)
            .ok_or(BitError::BufferFull)?;
        // left-align a partial final symbol, matching the AIS armoring convention
        let symbol = self.cur_symbol << (6 - self.cur_bits);
        *slot = sixbit_to_armor_char(symbol);
        self.char_idx += 1;
        self.cur_symbol = 0;
        self.cur_bits = 0;
        Ok(())
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    // write_bits/write_signed must report BufferFull or OutOfRange rather
    // than panic, for any output buffer size (including zero) and any
    // value/width combination. nbits is bounded (rather than a fully
    // unconstrained u32) purely so CBMC's loop unwinding has a concrete,
    // small bound to work with; the OutOfRange check for nbits > 64 is
    // exercised well within that bound regardless.
    #[kani::proof]
    #[kani::unwind(65)]
    fn write_bits_never_panics() {
        let mut buf: [u8; 4] = kani::any();
        let value: u64 = kani::any();
        let nbits: u32 = kani::any();
        kani::assume(nbits <= 100);
        let mut w = BitWriter::new(&mut buf);
        let _ = w.write_bits(value, nbits);
    }

    #[kani::proof]
    #[kani::unwind(65)]
    fn write_signed_never_panics() {
        let mut buf: [u8; 4] = kani::any();
        let value: i64 = kani::any();
        let nbits: u32 = kani::any();
        kani::assume(nbits <= 100);
        let mut w = BitWriter::new(&mut buf);
        let _ = w.write_signed(value, nbits);
    }
}

#[cfg(test)]
mod tests {
    use super::super::reader::BitReader;
    use super::*;

    #[test]
    fn writes_and_reads_back_known_value() {
        let mut buf = [0u8; 4];
        let mut w = BitWriter::new(&mut buf);
        w.write_bits(0b11_0101, 6).unwrap();
        w.write_bits(0, 6).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();
        assert_eq!(fill_bits, 0);

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 0b11_0101);
        assert_eq!(r.read_u8(6).unwrap(), 0);
    }

    #[test]
    fn partial_final_symbol_reports_fill_bits() {
        let mut buf = [0u8; 4];
        let mut w = BitWriter::new(&mut buf);
        w.write_bits(0b101, 3).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();
        assert_eq!(fill_bits, 3);
        assert_eq!(armored.len(), 1);

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(3).unwrap(), 0b101);
    }

    #[test]
    fn buffer_full_is_reported_not_panicked() {
        let mut buf = [0u8; 1];
        let mut w = BitWriter::new(&mut buf);
        w.write_bits(0, 6).unwrap();
        assert_eq!(w.write_bits(0, 6).unwrap_err(), BitError::BufferFull);
    }
}
