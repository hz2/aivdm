//! Error types for every layer of the decode/encode pipeline.

use core::fmt;

/// Errors from the bit-level reader/writer.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitError {
    /// Attempted to read past the end of the available bits.
    UnexpectedEof,
    /// A requested field width was zero or exceeded the return type's width.
    OutOfRange {
        /// Name of the field/operation that was out of range.
        field: &'static str,
    },
    /// The destination buffer has no room for more output.
    BufferFull,
}

impl fmt::Display for BitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of bit stream"),
            Self::OutOfRange { field } => write!(f, "field width out of range: {field}"),
            Self::BufferFull => write!(f, "output buffer is full"),
        }
    }
}

impl core::error::Error for BitError {}

/// Errors from NMEA 0183 sentence parsing.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NmeaError {
    /// The trailing `*hh` checksum did not match the computed checksum.
    ChecksumMismatch {
        /// Checksum given in the sentence.
        expected: u8,
        /// Checksum computed over the sentence body.
        actual: u8,
    },
    /// The sentence did not have the expected `!AIVDM`/`!AIVDO` structure.
    MalformedSentence,
    /// The talker/formatter was not `AIVDM`/`AIVDO`.
    UnsupportedFormatter,
    /// The comma-delimited field count did not match expectations.
    FieldCountMismatch,
    /// The fill-bits field was not a single digit in `0..=5`.
    InvalidFillBits,
    /// The armored payload contained a byte outside the six-bit ASCII alphabet.
    InvalidArmorChar,
}

impl fmt::Display for NmeaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "checksum mismatch: expected {expected:02X}, got {actual:02X}"
                )
            }
            Self::MalformedSentence => write!(f, "malformed NMEA sentence"),
            Self::UnsupportedFormatter => write!(f, "unsupported talker/formatter"),
            Self::FieldCountMismatch => write!(f, "unexpected number of comma-delimited fields"),
            Self::InvalidFillBits => write!(f, "invalid fill-bits field"),
            Self::InvalidArmorChar => write!(f, "invalid six-bit armor character"),
        }
    }
}

impl core::error::Error for NmeaError {}

/// Errors from multi-fragment sentence reassembly.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentError {
    /// A fragment arrived out of the expected 1..=count order.
    OutOfOrder,
    /// A fragment's sequential message ID did not match the in-progress reassembly.
    SequenceIdMismatch,
    /// A fragment's radio channel did not match the in-progress reassembly.
    ChannelMismatch,
    /// The reassembly buffer is too small to hold all fragments.
    BufferOverflow,
    /// The sentence declared more fragments than this assembler supports.
    TooManyFragments,
}

impl fmt::Display for FragmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfOrder => write!(f, "fragment received out of order"),
            Self::SequenceIdMismatch => write!(f, "fragment sequence id mismatch"),
            Self::ChannelMismatch => write!(f, "fragment channel mismatch"),
            Self::BufferOverflow => write!(f, "fragment reassembly buffer overflow"),
            Self::TooManyFragments => write!(f, "too many fragments for this assembler"),
        }
    }
}

impl core::error::Error for FragmentError {}

/// Errors from decoding/encoding a typed AIS message from/to its bit-packed payload.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageError {
    /// The 6-bit message type field did not match any known ITU-R M.1371 message.
    UnknownMessageType(u8),
    /// A bit-level read/write failed while decoding/encoding a message.
    Bit(BitError),
    /// A field held a value outside its legal enumerated range.
    InvalidEnumValue {
        /// Name of the offending field.
        field: &'static str,
        /// The raw value that was out of range.
        value: u32,
    },
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownMessageType(t) => write!(f, "unknown AIS message type: {t}"),
            Self::Bit(e) => write!(f, "{e}"),
            Self::InvalidEnumValue { field, value } => {
                write!(f, "invalid value {value} for field {field}")
            }
        }
    }
}

impl core::error::Error for MessageError {}

impl From<BitError> for MessageError {
    fn from(e: BitError) -> Self {
        Self::Bit(e)
    }
}

/// Top-level error type composing every layer, for `?`-based error propagation
/// from [`crate::decode_line`](crate::decode_line) and friends.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AisError {
    /// Error in the NMEA sentence layer.
    Nmea(NmeaError),
    /// Error in the fragment reassembly layer.
    Fragment(FragmentError),
    /// Error in the bit-level codec.
    Bit(BitError),
    /// Error in the message layer.
    Message(MessageError),
    /// The sentence is one fragment of a multi-part message; use
    /// [`crate::nmea::FragmentAssembler`] to reassemble it before decoding.
    IncompleteFragment,
}

impl fmt::Display for AisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nmea(e) => write!(f, "{e}"),
            Self::Fragment(e) => write!(f, "{e}"),
            Self::Bit(e) => write!(f, "{e}"),
            Self::Message(e) => write!(f, "{e}"),
            Self::IncompleteFragment => {
                write!(f, "sentence is one fragment of a multi-part message")
            }
        }
    }
}

impl core::error::Error for AisError {}

impl From<NmeaError> for AisError {
    fn from(e: NmeaError) -> Self {
        Self::Nmea(e)
    }
}

impl From<FragmentError> for AisError {
    fn from(e: FragmentError) -> Self {
        Self::Fragment(e)
    }
}

impl From<BitError> for AisError {
    fn from(e: BitError) -> Self {
        Self::Bit(e)
    }
}

impl From<MessageError> for AisError {
    fn from(e: MessageError) -> Self {
        Self::Message(e)
    }
}
