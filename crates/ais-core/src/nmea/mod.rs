//! NMEA 0183 sentence layer: checksum, `!AIVDM`/`!AIVDO` parsing, and
//! multi-fragment reassembly.

mod checksum;
mod fragment;
mod sentence;

pub use fragment::{CompleteMessage, FragmentAssembler};
pub use sentence::{Channel, Sentence};
