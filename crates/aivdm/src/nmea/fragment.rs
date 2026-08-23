//! Multi-fragment `!AIVDM`/`!AIVDO` sentence reassembly, without allocation.

use super::sentence::{Channel, Sentence};
use crate::error::FragmentError;

/// ITU-R M.1371 allows at most 9 fragments per multi-part message.
const MAX_FRAGMENTS: u8 = 9;

/// A fully reassembled multi-fragment payload, borrowed from the assembler
/// that produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompleteMessage<'a> {
    /// The concatenated six-bit ASCII-armored payload.
    pub armored: &'a [u8],
    /// Number of padding bits in the last armored character.
    pub fill_bits: u8,
    /// Radio channel the message was received on.
    pub channel: Channel,
}

/// Reassembles a sequence of sentence fragments sharing a sequential message
/// ID into a single armored payload, using a fixed-capacity buffer of `N`
/// bytes.
///
/// Single-fragment sentences (`fragment_count == 1`, the common case) do not
/// need this: read `Sentence::payload` directly instead.
///
/// Most callers reading a live/mixed feed want
/// [`LineDecoder`](crate::LineDecoder) instead, which wraps this type and
/// dispatches between the single- and multi-fragment paths internally.
#[derive(Debug)]
pub struct FragmentAssembler<const N: usize> {
    buf: [u8; N],
    len: usize,
    expected_fragments: u8,
    next_fragment: u8,
    seq_id: Option<u8>,
    channel: Channel,
    fill_bits: u8,
}

impl<const N: usize> Default for FragmentAssembler<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> FragmentAssembler<N> {
    /// Builds an empty assembler.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buf: [0u8; N],
            len: 0,
            expected_fragments: 0,
            next_fragment: 0,
            seq_id: None,
            channel: Channel::Other(0),
            fill_bits: 0,
        }
    }

    /// Discards any in-progress reassembly, e.g. after an error forces resync.
    pub fn reset(&mut self) {
        self.len = 0;
        self.expected_fragments = 0;
        self.next_fragment = 0;
        self.seq_id = None;
    }

    /// Feeds one sentence fragment into the assembler.
    ///
    /// Returns `Ok(Some(_))` once `sentence` completes the message,
    /// `Ok(None)` while more fragments are still expected, and `Err(_)` if
    /// the fragment does not fit the sequencing/capacity constraints (the
    /// caller should [`reset`](Self::reset) and resynchronize on error).
    ///
    /// # Errors
    /// See [`FragmentError`] variants for the specific sequencing/capacity
    /// failures this can report.
    pub fn push<'a>(
        &'a mut self,
        sentence: &Sentence<'_>,
    ) -> Result<Option<CompleteMessage<'a>>, FragmentError> {
        if sentence.fragment_count > MAX_FRAGMENTS {
            return Err(FragmentError::TooManyFragments);
        }

        if sentence.fragment_number == 1 {
            self.reset();
            self.expected_fragments = sentence.fragment_count;
            self.next_fragment = 1;
            self.seq_id = sentence.seq_id;
            self.channel = sentence.channel;
        } else {
            if sentence.fragment_number != self.next_fragment {
                return Err(FragmentError::OutOfOrder);
            }
            if sentence.seq_id != self.seq_id {
                return Err(FragmentError::SequenceIdMismatch);
            }
            if sentence.channel != self.channel {
                return Err(FragmentError::ChannelMismatch);
            }
        }

        let new_len = self.len + sentence.payload.len();
        if new_len > N {
            return Err(FragmentError::BufferOverflow);
        }
        self.buf[self.len..new_len].copy_from_slice(sentence.payload);
        self.len = new_len;
        self.fill_bits = sentence.fill_bits;
        self.next_fragment += 1;

        if sentence.fragment_number == sentence.fragment_count {
            Ok(Some(CompleteMessage {
                armored: &self.buf[..self.len],
                fill_bits: self.fill_bits,
                channel: self.channel,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frag(line: &str) -> Sentence<'_> {
        Sentence::parse(line).unwrap()
    }

    #[test]
    fn single_part_completes_immediately() {
        let mut a = FragmentAssembler::<64>::new();
        let s = frag("!AIVDM,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*5C");
        let complete = a.push(&s).unwrap().unwrap();
        assert_eq!(complete.armored, b"15M67FC000G?ufbE`FepT@3n00Sa");
    }

    #[test]
    fn two_part_reassembles_in_order() {
        // synthetic two-part split of the same payload with matching checksums
        let p1 = "!AIVDM,2,1,7,B,15M67FC000,0*6C";
        let p2 = "!AIVDM,2,2,7,B,0G?ufbE`FepT@3n00Sa,0*26";
        let mut a = FragmentAssembler::<64>::new();
        assert!(a.push(&frag(p1)).unwrap().is_none());
        let complete = a.push(&frag(p2)).unwrap().unwrap();
        assert_eq!(complete.armored, b"15M67FC0000G?ufbE`FepT@3n00Sa");
    }

    #[test]
    fn out_of_order_fragment_is_rejected() {
        let p1 = "!AIVDM,2,1,7,B,15M67FC000,0*6C";
        let p2 = "!AIVDM,2,2,7,B,0G?ufbE`FepT@3n00Sa,0*26";
        let mut a = FragmentAssembler::<64>::new();
        // feed second fragment first
        assert_eq!(a.push(&frag(p2)).unwrap_err(), FragmentError::OutOfOrder);
        assert!(a.push(&frag(p1)).unwrap().is_none());
    }

    #[test]
    fn buffer_overflow_is_reported() {
        let p1 = "!AIVDM,2,1,7,B,15M67FC000,0*6C";
        let p2 = "!AIVDM,2,2,7,B,0G?ufbE`FepT@3n00Sa,0*26";
        let mut a = FragmentAssembler::<15>::new();
        assert!(a.push(&frag(p1)).unwrap().is_none());
        assert_eq!(
            a.push(&frag(p2)).unwrap_err(),
            FragmentError::BufferOverflow
        );
    }
}
