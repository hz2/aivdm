//! Bit-level codec for AIS payload data.

mod reader;
mod writer;

pub use reader::BitReader;
pub use writer::BitWriter;

#[cfg(kani)]
mod kani_proofs {
    use super::{BitReader, BitWriter};

    // a single field's worth of six-bit armor chars: ceil(64/6) = 11, rounded
    // up generously so BufferFull is never the reason a write fails here.
    const BUF_LEN: usize = 16;

    #[kani::proof]
    #[kani::unwind(65)]
    fn write_read_u64_roundtrip() {
        let nbits: u32 = kani::any();
        kani::assume(nbits >= 1 && nbits <= 64);
        let value: u64 = kani::any();
        let masked = if nbits == 64 {
            value
        } else {
            value & ((1u64 << nbits) - 1)
        };

        let mut buf = [0u8; BUF_LEN];
        let mut w = BitWriter::new(&mut buf);
        w.write_bits(masked, nbits).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u64(nbits).unwrap(), masked);
    }

    #[kani::proof]
    #[kani::unwind(65)]
    fn write_read_signed_roundtrip() {
        let nbits: u32 = kani::any();
        kani::assume(nbits >= 1 && nbits <= 64);
        let value: i64 = kani::any();
        let min = if nbits == 64 {
            i64::MIN
        } else {
            -(1i64 << (nbits - 1))
        };
        let max = if nbits == 64 {
            i64::MAX
        } else {
            (1i64 << (nbits - 1)) - 1
        };
        kani::assume(value >= min && value <= max);

        let mut buf = [0u8; BUF_LEN];
        let mut w = BitWriter::new(&mut buf);
        w.write_signed(value, nbits).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_i64(nbits).unwrap(), value);
    }
}
