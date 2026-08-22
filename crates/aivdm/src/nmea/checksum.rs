//! NMEA 0183 XOR checksum.

/// Computes the NMEA XOR checksum over `body` (the sentence bytes between the
/// leading `!`/`$` and the trailing `*hh`, exclusive of both).
#[must_use]
pub fn compute(body: &[u8]) -> u8 {
    body.iter().fold(0u8, |acc, &b| acc ^ b)
}
