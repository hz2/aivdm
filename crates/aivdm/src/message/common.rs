//! Field types shared across multiple ITU-R M.1371 message types.

use crate::bits::{BitReader, BitWriter};
use crate::error::BitError;

/// Undecoded application-specific binary data, as carried by the DAC/FI
/// binary messages (types 6, 8, 25, 26).
///
/// This crate does not attempt to decode every IMO/regional-registered
/// application message; callers who need to interpret the payload can read
/// [`BinaryPayload::bits`] themselves against the relevant application
/// specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryPayload<const N: usize> {
    buf: [u8; N],
    bit_len: usize,
}

impl<const N: usize> BinaryPayload<N> {
    /// Number of meaningful bits stored.
    #[must_use]
    pub const fn bit_len(&self) -> usize {
        self.bit_len
    }

    /// The payload, packed MSB-first into bytes (the final byte is
    /// left-aligned if `bit_len` is not a multiple of 8).
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.buf[..self.bit_len.div_ceil(8)]
    }

    pub(crate) fn decode(r: &mut BitReader<'_>, bit_len: usize) -> Result<Self, BitError> {
        if bit_len.div_ceil(8) > N {
            return Err(BitError::OutOfRange {
                field: "BinaryPayload",
            });
        }
        let mut buf = [0u8; N];
        let mut remaining = bit_len;
        let mut i = 0;
        while remaining > 0 {
            let take = remaining.min(8);
            #[allow(
                clippy::cast_possible_truncation,
                reason = "take is clamped to 8 above, fits u32"
            )]
            let byte = r.read_u8(take as u32)?;
            buf[i] = byte << (8 - take);
            remaining -= take;
            i += 1;
        }
        Ok(Self { buf, bit_len })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        let mut remaining = self.bit_len;
        let mut i = 0;
        while remaining > 0 {
            let take = remaining.min(8);
            #[allow(
                clippy::cast_possible_truncation,
                reason = "take is clamped to 8 above, fits u32"
            )]
            let take_u32 = take as u32;
            let byte = self.buf[i] >> (8 - take);
            w.write_bits(u64::from(byte), take_u32)?;
            remaining -= take;
            i += 1;
        }
        Ok(())
    }

    /// Builds a payload directly from already MSB-first-packed bytes, for tests.
    #[cfg(test)]
    pub(crate) const fn test_from_raw(buf: [u8; N], bit_len: usize) -> Self {
        Self { buf, bit_len }
    }
}

/// A Maritime Mobile Service Identity: a 9-digit numeric station identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mmsi(u32);

impl Mmsi {
    /// Wraps a raw 30-bit MMSI value.
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// The raw numeric MMSI value.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl core::fmt::Display for Mmsi {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:09}", self.0)
    }
}

/// Raw sentinel meaning "longitude not available" (181 degrees, in 1/10000-minute units).
pub const LONGITUDE_NOT_AVAILABLE_RAW: i32 = 181 * 60 * 10_000;
/// Raw sentinel meaning "latitude not available" (91 degrees, in 1/10000-minute units).
pub const LATITUDE_NOT_AVAILABLE_RAW: i32 = 91 * 60 * 10_000;

/// Longitude in 1/10000-minute units (signed 28-bit field in the wire format).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Longitude(i32);

impl Longitude {
    /// Wraps a raw signed longitude value in 1/10000-minute units.
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    /// The raw signed value, in 1/10000-minute units.
    #[must_use]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Whether this field carries an actual fix rather than the "not available" sentinel.
    #[must_use]
    pub const fn is_available(self) -> bool {
        self.0 != LONGITUDE_NOT_AVAILABLE_RAW
    }

    /// The longitude in decimal degrees, or `None` if not available.
    #[must_use]
    pub fn as_degrees(self) -> Option<f64> {
        self.is_available().then(|| f64::from(self.0) / 600_000.0)
    }
}

/// Latitude in 1/10000-minute units (signed 27-bit field in the wire format).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Latitude(i32);

impl Latitude {
    /// Wraps a raw signed latitude value in 1/10000-minute units.
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    /// The raw signed value, in 1/10000-minute units.
    #[must_use]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Whether this field carries an actual fix rather than the "not available" sentinel.
    #[must_use]
    pub const fn is_available(self) -> bool {
        self.0 != LATITUDE_NOT_AVAILABLE_RAW
    }

    /// The latitude in decimal degrees, or `None` if not available.
    #[must_use]
    pub fn as_degrees(self) -> Option<f64> {
        self.is_available().then(|| f64::from(self.0) / 600_000.0)
    }
}

/// Speed over ground, in units of 0.1 knot (10-bit field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sog(u16);

impl Sog {
    /// Raw sentinel meaning "not available".
    pub const NOT_AVAILABLE_RAW: u16 = 1023;
    /// Raw sentinel meaning "102.2 knots or higher".
    pub const HIGH_SPEED_RAW: u16 = 1022;

    /// Wraps a raw tenths-of-a-knot value.
    #[must_use]
    pub const fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    /// The raw tenths-of-a-knot value.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Whether this field carries an actual speed rather than the "not available" sentinel.
    #[must_use]
    pub const fn is_available(self) -> bool {
        self.0 != Self::NOT_AVAILABLE_RAW
    }

    /// The speed in knots, or `None` if not available.
    #[must_use]
    pub fn knots(self) -> Option<f64> {
        self.is_available().then(|| f64::from(self.0) / 10.0)
    }
}

/// Course over ground, in units of 0.1 degree (12-bit field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cog(u16);

impl Cog {
    /// Raw sentinel meaning "not available".
    pub const NOT_AVAILABLE_RAW: u16 = 3600;

    /// Wraps a raw tenths-of-a-degree value.
    #[must_use]
    pub const fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    /// The raw tenths-of-a-degree value.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Whether this field carries an actual course rather than the "not available" sentinel.
    #[must_use]
    pub const fn is_available(self) -> bool {
        self.0 != Self::NOT_AVAILABLE_RAW
    }

    /// The course in decimal degrees, or `None` if not available.
    #[must_use]
    pub fn degrees(self) -> Option<f64> {
        self.is_available().then(|| f64::from(self.0) / 10.0)
    }
}

/// True heading, in whole degrees (9-bit field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heading(u16);

impl Heading {
    /// Raw sentinel meaning "not available".
    pub const NOT_AVAILABLE_RAW: u16 = 511;

    /// Wraps a raw whole-degree heading value.
    #[must_use]
    pub const fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    /// The raw whole-degree value.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Whether this field carries an actual heading rather than the "not available" sentinel.
    #[must_use]
    pub const fn is_available(self) -> bool {
        self.0 != Self::NOT_AVAILABLE_RAW
    }

    /// The heading in whole degrees, or `None` if not available.
    #[must_use]
    pub const fn degrees(self) -> Option<u16> {
        if self.is_available() {
            Some(self.0)
        } else {
            None
        }
    }
}

/// Rate of turn, ITU-R M.1371-encoded (signed 8-bit field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateOfTurn(i8);

impl RateOfTurn {
    /// Wraps a raw ITU-R M.1371-encoded rate-of-turn value.
    #[must_use]
    pub const fn from_raw(raw: i8) -> Self {
        Self(raw)
    }

    /// The raw encoded value.
    #[must_use]
    pub const fn raw(self) -> i8 {
        self.0
    }

    /// Whether a rate of turn indicator is present at all.
    #[must_use]
    pub const fn is_available(self) -> bool {
        self.0 != -128
    }

    /// The decoded rate of turn in degrees/minute, or `None` if unavailable,
    /// or if the sensor only reported "turning right/left at more than
    /// 5deg/30s" without an exact rate (raw value `+127`/`-127`).
    #[must_use]
    pub fn degrees_per_minute(self) -> Option<f64> {
        if !self.is_available() || self.0 == 127 || self.0 == -127 {
            return None;
        }
        let x = f64::from(self.0) / 4.733;
        Some(x * x.abs())
    }
}

/// Navigational status (4-bit field), ITU-R M.1371 Table 45.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NavigationStatus {
    /// Under way using engine.
    UnderWayUsingEngine,
    /// At anchor.
    AtAnchor,
    /// Not under command.
    NotUnderCommand,
    /// Restricted maneuverability.
    RestrictedManoeuvrability,
    /// Constrained by her draught.
    ConstrainedByDraught,
    /// Moored.
    Moored,
    /// Aground.
    Aground,
    /// Engaged in fishing.
    EngagedInFishing,
    /// Under way sailing.
    UnderWaySailing,
    /// Reserved for high-speed craft.
    ReservedHsc,
    /// Reserved for wing-in-ground craft.
    ReservedWig,
    /// Reserved (regional: power-driven vessel towing astern).
    ReservedTowingAstern,
    /// Reserved (regional: power-driven vessel pushing ahead/towing alongside).
    ReservedPushingAhead,
    /// Reserved for future use.
    Reserved13,
    /// AIS-SART (active), AIS-MOB, or AIS-EPIRB.
    AisSartMobEpirb,
    /// Not defined (default).
    NotDefined,
}

impl NavigationStatus {
    /// Decodes a raw 4-bit navigational status value.
    #[must_use]
    pub const fn from_raw(v: u8) -> Self {
        match v {
            0 => Self::UnderWayUsingEngine,
            1 => Self::AtAnchor,
            2 => Self::NotUnderCommand,
            3 => Self::RestrictedManoeuvrability,
            4 => Self::ConstrainedByDraught,
            5 => Self::Moored,
            6 => Self::Aground,
            7 => Self::EngagedInFishing,
            8 => Self::UnderWaySailing,
            9 => Self::ReservedHsc,
            10 => Self::ReservedWig,
            11 => Self::ReservedTowingAstern,
            12 => Self::ReservedPushingAhead,
            13 => Self::Reserved13,
            14 => Self::AisSartMobEpirb,
            _ => Self::NotDefined,
        }
    }

    /// Encodes back to the raw 4-bit navigational status value.
    #[must_use]
    pub const fn to_raw(self) -> u8 {
        match self {
            Self::UnderWayUsingEngine => 0,
            Self::AtAnchor => 1,
            Self::NotUnderCommand => 2,
            Self::RestrictedManoeuvrability => 3,
            Self::ConstrainedByDraught => 4,
            Self::Moored => 5,
            Self::Aground => 6,
            Self::EngagedInFishing => 7,
            Self::UnderWaySailing => 8,
            Self::ReservedHsc => 9,
            Self::ReservedWig => 10,
            Self::ReservedTowingAstern => 11,
            Self::ReservedPushingAhead => 12,
            Self::Reserved13 => 13,
            Self::AisSartMobEpirb => 14,
            Self::NotDefined => 15,
        }
    }
}

/// Special maneuver indicator (2-bit field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ManeuverIndicator {
    /// Not available.
    NotAvailable,
    /// No special maneuver.
    NoSpecialManeuver,
    /// Special maneuver (e.g. regional passing arrangement).
    SpecialManeuver,
    /// Reserved value.
    Reserved,
}

impl ManeuverIndicator {
    /// Decodes a raw 2-bit maneuver indicator value.
    #[must_use]
    pub const fn from_raw(v: u8) -> Self {
        match v {
            0 => Self::NotAvailable,
            1 => Self::NoSpecialManeuver,
            2 => Self::SpecialManeuver,
            _ => Self::Reserved,
        }
    }

    /// Encodes back to the raw 2-bit maneuver indicator value.
    #[must_use]
    pub const fn to_raw(self) -> u8 {
        match self {
            Self::NotAvailable => 0,
            Self::NoSpecialManeuver => 1,
            Self::SpecialManeuver => 2,
            Self::Reserved => 3,
        }
    }
}

/// UTC second timestamp (6-bit field) as used in position reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Timestamp {
    /// The UTC second (`0..=59`) at which the report was generated.
    Second(u8),
    /// Positioning system time stamp is not available.
    NotAvailable,
    /// Positioning system is in manual input mode.
    ManualInputMode,
    /// Positioning system is in dead reckoning mode.
    DeadReckoning,
    /// Positioning system is inoperative.
    Inoperative,
}

impl Timestamp {
    /// Decodes a raw 6-bit timestamp value.
    #[must_use]
    pub const fn from_raw(v: u8) -> Self {
        match v {
            0..=59 => Self::Second(v),
            61 => Self::ManualInputMode,
            62 => Self::DeadReckoning,
            63 => Self::Inoperative,
            _ => Self::NotAvailable,
        }
    }

    /// Encodes back to the raw 6-bit timestamp value.
    #[must_use]
    pub const fn to_raw(self) -> u8 {
        match self {
            Self::Second(s) => s,
            Self::NotAvailable => 60,
            Self::ManualInputMode => 61,
            Self::DeadReckoning => 62,
            Self::Inoperative => 63,
        }
    }
}

/// Electronic Position Fixing Device type (4-bit field), ITU-R M.1371 Table 47.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EpfdType {
    /// Not specified / undefined.
    Undefined,
    /// GPS.
    Gps,
    /// GLONASS.
    Glonass,
    /// Combined GPS/GLONASS.
    GpsGlonass,
    /// Loran-C.
    LoranC,
    /// Chayka.
    Chayka,
    /// Integrated navigation system.
    IntegratedNavigationSystem,
    /// Surveyed (fixed position, e.g. a base station).
    Surveyed,
    /// Galileo.
    Galileo,
    /// Reserved/unused value.
    Reserved(u8),
}

impl EpfdType {
    /// Decodes a raw 4-bit EPFD type value.
    #[must_use]
    pub const fn from_raw(v: u8) -> Self {
        match v {
            1 => Self::Gps,
            2 => Self::Glonass,
            3 => Self::GpsGlonass,
            4 => Self::LoranC,
            5 => Self::Chayka,
            6 => Self::IntegratedNavigationSystem,
            7 => Self::Surveyed,
            8 => Self::Galileo,
            0 => Self::Undefined,
            other => Self::Reserved(other),
        }
    }

    /// Encodes back to the raw 4-bit EPFD type value.
    #[must_use]
    pub const fn to_raw(self) -> u8 {
        match self {
            Self::Undefined => 0,
            Self::Gps => 1,
            Self::Glonass => 2,
            Self::GpsGlonass => 3,
            Self::LoranC => 4,
            Self::Chayka => 5,
            Self::IntegratedNavigationSystem => 6,
            Self::Surveyed => 7,
            Self::Galileo => 8,
            Self::Reserved(v) => v,
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::string::ToString;

    #[test]
    fn mmsi_displays_zero_padded() {
        assert_eq!(Mmsi::from_raw(123).to_string(), "000000123");
    }

    #[test]
    fn navigation_status_roundtrips() {
        for v in 0u8..16 {
            assert_eq!(NavigationStatus::from_raw(v).to_raw(), v);
        }
    }

    #[test]
    fn maneuver_indicator_roundtrips() {
        for v in 0u8..4 {
            assert_eq!(ManeuverIndicator::from_raw(v).to_raw(), v);
        }
    }

    #[test]
    fn timestamp_roundtrips() {
        for v in 0u8..64 {
            assert_eq!(Timestamp::from_raw(v).to_raw(), v);
        }
    }

    #[test]
    fn epfd_type_roundtrips() {
        for v in 0u8..16 {
            assert_eq!(EpfdType::from_raw(v).to_raw(), v);
        }
    }

    #[test]
    fn longitude_not_available_sentinel() {
        let lon = Longitude::from_raw(LONGITUDE_NOT_AVAILABLE_RAW);
        assert!(!lon.is_available());
        assert_eq!(lon.as_degrees(), None);
    }

    #[test]
    fn longitude_known_value() {
        // -73.5 degrees == -73.5 * 600_000
        let lon = Longitude::from_raw(-44_100_000);
        assert_eq!(lon.as_degrees(), Some(-73.5));
    }
}
