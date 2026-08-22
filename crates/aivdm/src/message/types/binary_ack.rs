//! Binary Acknowledge and Safety Related Acknowledge — message types 7 and 13.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::Mmsi;

/// One acknowledged destination MMSI and the sequence number being acked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ack {
    /// MMSI of the station whose message is being acknowledged.
    pub mmsi: Mmsi,
    /// Sequence number being acknowledged (0..=3).
    pub sequence_number: u8,
}

/// Binary Acknowledge (type 7) or Safety Related Acknowledge (type 13).
///
/// Both message types share an identical wire layout (72..=168 bits,
/// acknowledging 1-4 stations); they differ only in whether they acknowledge
/// a preceding binary (type 6) or safety-related (type 12) message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Acknowledge {
    /// Which of message types 7 or 13 this is.
    pub message_type: u8,
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Acknowledged destinations (up to 4; unused slots are `None`).
    pub acks: [Option<Ack>; 4],
}

impl Acknowledge {
    pub(crate) fn decode(message_type: u8, r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        r.skip(2)?; // spare

        let mut acks = [None; 4];
        for ack in &mut acks {
            if r.remaining_bits() < 32 {
                break;
            }
            let ack_mmsi = Mmsi::from_raw(r.read_u32(30)?);
            let sequence_number = r.read_u8(2)?;
            *ack = Some(Ack {
                mmsi: ack_mmsi,
                sequence_number,
            });
        }

        Ok(Self {
            message_type,
            repeat_indicator,
            mmsi,
            acks,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(u64::from(self.message_type), 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(0, 2)?; // spare
        for ack in self.acks.into_iter().flatten() {
            w.write_bits(u64::from(ack.mmsi.raw()), 30)?;
            w.write_bits(u64::from(ack.sequence_number), 2)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(message_type: u8) -> Acknowledge {
        Acknowledge {
            message_type,
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(366_053_209),
            acks: [
                Some(Ack {
                    mmsi: Mmsi::from_raw(111_111_111),
                    sequence_number: 1,
                }),
                Some(Ack {
                    mmsi: Mmsi::from_raw(222_222_222),
                    sequence_number: 2,
                }),
                None,
                None,
            ],
        }
    }

    fn round_trip(original: Acknowledge) -> Acknowledge {
        let mut buf = [0u8; 32];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), original.message_type);
        Acknowledge::decode(original.message_type, &mut r).unwrap()
    }

    #[test]
    fn round_trips_as_type_7() {
        let original = sample(7);
        assert_eq!(round_trip(original), original);
    }

    #[test]
    fn round_trips_as_type_13() {
        let original = sample(13);
        assert_eq!(round_trip(original), original);
    }
}
