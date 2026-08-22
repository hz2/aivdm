//! Data Link Management Message — message type 20.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::Mmsi;

/// One reserved-slot block within a Data Link Management Message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotReservation {
    /// Slot offset from the start of the frame.
    pub offset: u16,
    /// Number of consecutive slots reserved (1..=5).
    pub reserved_slots: u8,
    /// Number of frames the reservation remains valid for (1..=7).
    pub timeout: u8,
    /// Slot increment applied at each following frame.
    pub increment: u16,
}

/// Data Link Management Message (message type 20, up to 4 reservation blocks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataLinkManagement {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI (a base station).
    pub mmsi: Mmsi,
    /// Reserved-slot blocks (up to 4; unused slots are `None`).
    pub reservations: [Option<SlotReservation>; 4],
}

impl DataLinkManagement {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        r.skip(2)?; // spare

        let mut reservations = [None; 4];
        for slot in &mut reservations {
            if r.remaining_bits() < 30 {
                break;
            }
            let offset = r.read_u16(12)?;
            let reserved_slots = r.read_u8(4)?;
            let timeout = r.read_u8(3)?;
            let increment = r.read_u16(11)?;
            *slot = Some(SlotReservation {
                offset,
                reserved_slots,
                timeout,
                increment,
            });
        }

        Ok(Self {
            repeat_indicator,
            mmsi,
            reservations,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(20, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(0, 2)?; // spare
        for reservation in self.reservations.into_iter().flatten() {
            w.write_bits(u64::from(reservation.offset), 12)?;
            w.write_bits(u64::from(reservation.reserved_slots), 4)?;
            w.write_bits(u64::from(reservation.timeout), 3)?;
            w.write_bits(u64::from(reservation.increment), 11)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_with_two_reservations() {
        let original = DataLinkManagement {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(3_669_708),
            reservations: [
                Some(SlotReservation {
                    offset: 100,
                    reserved_slots: 2,
                    timeout: 3,
                    increment: 50,
                }),
                Some(SlotReservation {
                    offset: 200,
                    reserved_slots: 1,
                    timeout: 5,
                    increment: 75,
                }),
                None,
                None,
            ],
        };

        let mut buf = [0u8; 32];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 20);
        let decoded = DataLinkManagement::decode(&mut r).unwrap();
        assert_eq!(decoded, original);
    }
}
