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
}
