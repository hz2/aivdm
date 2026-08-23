//! A `no_std`, allocation-free parser and encoder for AIS (Automatic
//! Identification System) messages, per ITU-R M.1371, as carried over NMEA
//! 0183 `!AIVDM`/`!AIVDO` sentences.
//!
//! # Layering
//!
//! - [`nmea`] parses and checksum-validates `!AIVDM`/`!AIVDO` sentences and
//!   reassembles multi-fragment messages into a single armored payload.
//! - [`bits`] reads and writes arbitrary-width fields directly against the
//!   six-bit ASCII-armored payload bytes; there is no separate "unpacked
//!   bitstream" buffer, since AIS fields do not align to byte boundaries.
//! - [`message`] decodes/encodes the bit-packed payload into a typed
//!   [`AisMessage`], one variant per ITU-R M.1371 message type.
//!
//! # No allocation
//!
//! Nothing in this crate uses `alloc`. Multi-fragment reassembly
//! ([`nmea::FragmentAssembler`]) uses a fixed-capacity, caller-sized buffer
//! instead of a growable one, so it works with no allocator at all.
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bits;
pub mod error;
pub mod message;
pub mod nmea;
pub mod string;

pub use error::AisError;
pub use message::AisMessage;
pub use message::common::*;
pub use nmea::{Channel, CompleteMessage, FragmentAssembler, Sentence};

/// Decodes a single, complete NMEA line into a typed AIS message.
///
/// This only handles single-fragment sentences (`fragment_count == 1`),
/// which cover the large majority of AIS traffic. For a sentence that is one
/// fragment of a multi-part message, this returns
/// [`AisError::IncompleteFragment`]; parse the line with [`Sentence::parse`]
/// instead and feed the fragments through a [`nmea::FragmentAssembler`]
/// yourself, then call [`decode_payload`] on the reassembled payload.
///
/// # Errors
/// Returns an [`AisError`] if the line fails NMEA parsing, is one fragment
/// of a multi-fragment sentence, or its payload fails to decode as a known
/// message type.
pub fn decode_line(line: &str) -> Result<AisMessage, AisError> {
    let sentence = Sentence::parse(line)?;
    if sentence.fragment_count != 1 {
        return Err(AisError::IncompleteFragment);
    }
    decode_payload(sentence.payload, sentence.fill_bits)
}

/// Decodes a typed AIS message from an already-assembled six-bit
/// ASCII-armored payload.
///
/// # Errors
/// Returns an [`AisError`] if the payload fails to decode as a known message type.
pub fn decode_payload(armored: &[u8], fill_bits: u8) -> Result<AisMessage, AisError> {
    let mut reader = bits::BitReader::new(armored, fill_bits);
    AisMessage::decode(&mut reader).map_err(AisError::from)
}

/// Encodes a typed AIS message into `buf` as a six-bit ASCII-armored payload.
///
/// Returns the armored slice of `buf` that was written, and the number of
/// fill bits needed in the NMEA sentence's fill-bits field.
///
/// # Errors
/// Returns an [`AisError`] if `buf` is too small to hold the encoded payload.
pub fn encode_payload<'a>(
    message: &AisMessage,
    buf: &'a mut [u8],
) -> Result<(&'a [u8], u8), AisError> {
    let mut writer = bits::BitWriter::new(buf);
    message.encode(&mut writer)?;
    Ok(writer.finish()?)
}

/// The largest armored payload [`encode_line`] will encode a message into.
/// Generous enough for every message type that realistically fits in a
/// single NMEA sentence (see [`encode_line`]'s docs); comfortably smaller
/// than the stack budget of even small embedded targets.
const MAX_SINGLE_SENTENCE_PAYLOAD_BYTES: usize = 128;

/// Encodes a typed AIS message into a single, complete `!AIVDM`/`!AIVDO`
/// NMEA line on the given `channel`, writing into `buf` and returning the
/// written slice. Set `is_own_ship` to emit `!AIVDO` (a simulated/own-ship
/// report) rather than `!AIVDM`.
///
/// This only handles messages that fit in a single sentence, mirroring
/// [`decode_line`]'s single-fragment restriction: large message types (5,
/// 12, 22, 24, ...) that real-world transmitters split across multiple
/// sentences are not supported by this convenience function. For those,
/// encode the payload with [`encode_payload`], split it into chunks
/// yourself, and format each fragment with [`Sentence::format`].
///
/// The returned line has no trailing line ending (e.g. `\r\n`); add your
/// transport's convention yourself.
///
/// # Errors
/// Returns an [`AisError`] if the message fails to encode, if its armored
/// payload does not fit in a single sentence, or if `buf` is too small to
/// hold the formatted line.
pub fn encode_line<'a>(
    message: &AisMessage,
    channel: Channel,
    is_own_ship: bool,
    buf: &'a mut [u8],
) -> Result<&'a [u8], AisError> {
    let mut payload_buf = [0u8; MAX_SINGLE_SENTENCE_PAYLOAD_BYTES];
    let (payload, fill_bits) = encode_payload(message, &mut payload_buf)?;

    let sentence = Sentence {
        is_own_ship,
        fragment_count: 1,
        fragment_number: 1,
        seq_id: None,
        channel,
        payload,
        fill_bits,
    };
    Ok(sentence.format(buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_a_real_position_report() {
        let msg = decode_line("!AIVDM,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*5C").unwrap();
        assert_eq!(msg.message_type(), 1);
        let AisMessage::PositionReportClassA(p) = msg else {
            panic!("expected PositionReportClassA");
        };
        assert_eq!(p.mmsi.raw(), 366_053_209);
        assert_eq!(
            p.navigation_status,
            NavigationStatus::RestrictedManoeuvrability
        );
        assert!((p.longitude.as_degrees().unwrap() - (-122.341_618_333)).abs() < 1e-6);
        assert!((p.latitude.as_degrees().unwrap() - 37.802_118_333).abs() < 1e-6);
    }

    #[test]
    fn common_accessors_work_directly_on_ais_message() {
        let msg = decode_line("!AIVDM,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*5C").unwrap();
        assert_eq!(msg.mmsi().raw(), 366_053_209);
        assert_eq!(
            msg.navigation_status(),
            Some(NavigationStatus::RestrictedManoeuvrability)
        );
        let position = msg.position().unwrap();
        assert!((position.latitude - 37.802_118_333).abs() < 1e-6);
        assert!((position.longitude - (-122.341_618_333)).abs() < 1e-6);
    }

    #[test]
    fn multi_fragment_sentence_reports_incomplete() {
        let err = decode_line("!AIVDM,2,1,7,B,15M67FC000,0*6C").unwrap_err();
        assert_eq!(err, AisError::IncompleteFragment);
    }

    #[test]
    fn encode_round_trips_a_real_position_report() {
        let original_payload = b"15M67FC000G?ufbE`FepT@3n00Sa";
        let msg = decode_payload(original_payload, 0).unwrap();

        let mut buf = [0u8; 32];
        let (armored, fill_bits) = encode_payload(&msg, &mut buf).unwrap();
        assert_eq!(fill_bits, 0);
        assert_eq!(armored, original_payload);

        let re_decoded = decode_payload(armored, fill_bits).unwrap();
        assert_eq!(re_decoded, msg);
    }

    #[test]
    fn encode_line_round_trips_a_real_position_report() {
        let line = "!AIVDM,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*5C";
        let msg = decode_line(line).unwrap();

        let mut buf = [0u8; 64];
        let encoded = encode_line(&msg, Channel::B, false, &mut buf).unwrap();
        assert_eq!(encoded, line.as_bytes());

        let re_decoded = decode_line(core::str::from_utf8(encoded).unwrap()).unwrap();
        assert_eq!(re_decoded, msg);
    }

    #[test]
    fn encode_line_emits_aivdo_for_own_ship() {
        let line = "!AIVDM,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*5C";
        let msg = decode_line(line).unwrap();

        let mut buf = [0u8; 64];
        let encoded = encode_line(&msg, Channel::A, true, &mut buf).unwrap();
        assert!(encoded.starts_with(b"!AIVDO,"));

        let reparsed = Sentence::parse(core::str::from_utf8(encoded).unwrap()).unwrap();
        assert!(reparsed.is_own_ship);
        assert_eq!(reparsed.channel, Channel::A);
    }
}
