//! Interrogation — message type 15.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::Mmsi;

/// One requested message type and the slot offset it should be sent at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageRequest {
    /// ITU-R M.1371 message type being requested.
    pub message_type: u8,
    /// Slot offset at which the reply should be sent.
    pub slot_offset: u16,
}

/// A second interrogated station and its (single) message request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecondStation {
    /// MMSI of the second interrogated station.
    pub mmsi: Mmsi,
    /// The message requested from it.
    pub request: MessageRequest,
}

/// Interrogation (message type 15, 88..=160 bits).
///
/// Asks one or two stations to send a specific message type at a given slot
/// offset; the first interrogated station may be asked for up to two
/// message types, the second for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interrogation {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// MMSI of the first interrogated station.
    pub station_1_mmsi: Mmsi,
    /// First message requested from the first station.
    pub station_1_request_1: MessageRequest,
    /// Second message requested from the first station, if any.
    pub station_1_request_2: Option<MessageRequest>,
    /// Second interrogated station and its request, if any.
    pub station_2: Option<SecondStation>,
}

impl Interrogation {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        r.skip(2)?; // spare
        let station_1_mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let station_1_request_1 = decode_request(r)?;

        // Each of the following chunks is independently optional: real-world
        // producers are inconsistent about whether trailing spare bits are
        // present at each cut point (there is a known design ambiguity in
        // the ITU-R M.1371 spec here), so every piece is guarded on its own
        // rather than assuming a fixed combined width.
        if r.remaining_bits() >= 2 {
            r.skip(2)?; // spare
        }

        let station_1_request_2 = if r.remaining_bits() >= 18 {
            Some(decode_request(r)?)
        } else {
            None
        };

        if r.remaining_bits() >= 2 {
            r.skip(2)?; // spare
        }

        let station_2 = if r.remaining_bits() >= 48 {
            let station_mmsi = Mmsi::from_raw(r.read_u32(30)?);
            let request = decode_request(r)?;
            Some(SecondStation {
                mmsi: station_mmsi,
                request,
            })
        } else {
            None
        };

        if r.remaining_bits() >= 2 {
            r.skip(2)?; // spare
        }

        Ok(Self {
            repeat_indicator,
            mmsi,
            station_1_mmsi,
            station_1_request_1,
            station_1_request_2,
            station_2,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(15, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(0, 2)?; // spare
        w.write_bits(u64::from(self.station_1_mmsi.raw()), 30)?;
        encode_request(self.station_1_request_1, w)?;

        // station_2 can only be positioned correctly if the request_2 slot
        // ahead of it is also written, even if request_2 itself is absent.
        let write_request_2 = self.station_1_request_2.is_some() || self.station_2.is_some();
        if write_request_2 {
            w.write_bits(0, 2)?; // spare
            let request = self.station_1_request_2.unwrap_or(MessageRequest {
                message_type: 0,
                slot_offset: 0,
            });
            encode_request(request, w)?;
        }

        if let Some(station_2) = self.station_2 {
            w.write_bits(0, 2)?; // spare
            w.write_bits(u64::from(station_2.mmsi.raw()), 30)?;
            encode_request(station_2.request, w)?;
        }

        Ok(())
    }
}

fn decode_request(r: &mut BitReader<'_>) -> Result<MessageRequest, MessageError> {
    let message_type = r.read_u8(6)?;
    let slot_offset = r.read_u16(12)?;
    Ok(MessageRequest {
        message_type,
        slot_offset,
    })
}

fn encode_request(request: MessageRequest, w: &mut BitWriter<'_>) -> Result<(), BitError> {
    w.write_bits(u64::from(request.message_type), 6)?;
    w.write_bits(u64::from(request.slot_offset), 12)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(
        station_1_request_2: Option<MessageRequest>,
        station_2: Option<SecondStation>,
    ) -> Interrogation {
        Interrogation {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(366_053_209),
            station_1_mmsi: Mmsi::from_raw(366_999_999),
            station_1_request_1: MessageRequest {
                message_type: 5,
                slot_offset: 100,
            },
            station_1_request_2,
            station_2,
        }
    }

    fn round_trip(original: Interrogation) -> Interrogation {
        let mut buf = [0u8; 32];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 15);
        Interrogation::decode(&mut r).unwrap()
    }

    #[test]
    fn round_trips_minimal() {
        let original = sample(None, None);
        assert_eq!(round_trip(original), original);
    }

    #[test]
    fn round_trips_with_second_request_and_station() {
        let original = sample(
            Some(MessageRequest {
                message_type: 18,
                slot_offset: 50,
            }),
            Some(SecondStation {
                mmsi: Mmsi::from_raw(111_222_333),
                request: MessageRequest {
                    message_type: 24,
                    slot_offset: 75,
                },
            }),
        );
        assert_eq!(round_trip(original), original);
    }
}
