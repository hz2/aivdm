//! Assignment Mode Command — message type 16.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::Mmsi;

/// One assignment target within an Assignment Mode Command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assignment {
    /// MMSI of the assigned station.
    pub destination_mmsi: Mmsi,
    /// Slot offset at which the assigned station should start reporting.
    pub offset: u16,
    /// Reporting interval to assign, in slots (0 = use the autonomous default).
    pub increment: u16,
}

/// Assignment Mode Command (message type 16, 96 or 144 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignmentModeCommand {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI (a base station).
    pub mmsi: Mmsi,
    /// First assignment target.
    pub first: Assignment,
    /// Second assignment target, if present.
    pub second: Option<Assignment>,
}

impl AssignmentModeCommand {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        r.skip(2)?; // spare
        let first = decode_assignment(r)?;
        let second = if r.remaining_bits() >= 52 {
            Some(decode_assignment(r)?)
        } else {
            None
        };

        Ok(Self {
            repeat_indicator,
            mmsi,
            first,
            second,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(16, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(0, 2)?; // spare
        encode_assignment(self.first, w)?;
        if let Some(second) = self.second {
            encode_assignment(second, w)?;
        }
        Ok(())
    }
}

fn decode_assignment(r: &mut BitReader<'_>) -> Result<Assignment, MessageError> {
    let destination_mmsi = Mmsi::from_raw(r.read_u32(30)?);
    let offset = r.read_u16(12)?;
    let increment = r.read_u16(10)?;
    Ok(Assignment {
        destination_mmsi,
        offset,
        increment,
    })
}

fn encode_assignment(a: Assignment, w: &mut BitWriter<'_>) -> Result<(), BitError> {
    w.write_bits(u64::from(a.destination_mmsi.raw()), 30)?;
    w.write_bits(u64::from(a.offset), 12)?;
    w.write_bits(u64::from(a.increment), 10)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(second: Option<Assignment>) -> AssignmentModeCommand {
        AssignmentModeCommand {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(3_669_708),
            first: Assignment {
                destination_mmsi: Mmsi::from_raw(366_053_209),
                offset: 100,
                increment: 5,
            },
            second,
        }
    }

    fn round_trip(original: AssignmentModeCommand) -> AssignmentModeCommand {
        let mut buf = [0u8; 32];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 16);
        AssignmentModeCommand::decode(&mut r).unwrap()
    }

    #[test]
    fn round_trips_with_one_destination() {
        let original = sample(None);
        assert_eq!(round_trip(original), original);
    }

    #[test]
    fn round_trips_with_two_destinations() {
        let original = sample(Some(Assignment {
            destination_mmsi: Mmsi::from_raw(366_999_999),
            offset: 200,
            increment: 8,
        }));
        assert_eq!(round_trip(original), original);
    }
}
