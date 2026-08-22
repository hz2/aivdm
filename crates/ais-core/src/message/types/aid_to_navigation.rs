//! Aid-to-Navigation Report — message type 21.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::{EpfdType, Latitude, Longitude, Mmsi, Timestamp};
use crate::string::FixedStr;

/// Aid-to-Navigation Report (message type 21, 272..=360 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent single-bit wire field, not related state"
)]
pub struct AidToNavigationReport {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Type of aid to navigation (raw code, ITU-R M.1371 Table 20).
    pub aid_type: u8,
    /// Name of the aid to navigation (including any name extension characters).
    pub name: FixedStr<34>,
    /// Whether the reported position has better than 10m DGPS-quality accuracy.
    pub position_accuracy: bool,
    /// Longitude.
    pub longitude: Longitude,
    /// Latitude.
    pub latitude: Latitude,
    /// Distance from the reference point to the bow, in meters.
    pub dimension_to_bow: u16,
    /// Distance from the reference point to the stern, in meters.
    pub dimension_to_stern: u16,
    /// Distance from the reference point to the port side, in meters.
    pub dimension_to_port: u8,
    /// Distance from the reference point to the starboard side, in meters.
    pub dimension_to_starboard: u8,
    /// Electronic position fixing device type.
    pub epfd_type: EpfdType,
    /// UTC second timestamp.
    pub timestamp: Timestamp,
    /// Whether the aid to navigation is off its charted position.
    pub off_position: bool,
    /// Whether the RAIM (Receiver Autonomous Integrity Monitoring) flag is set.
    pub raim: bool,
    /// Whether this is a virtual (non-physical) aid to navigation.
    pub virtual_aid: bool,
    /// Whether the station is assigned by a message 16 or 22.
    pub assigned: bool,
}

impl AidToNavigationReport {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let aid_type = r.read_u8(5)?;
        let name = r.read_sixbit_ascii::<34>(20)?;
        let position_accuracy = r.read_bool()?;
        let longitude = Longitude::from_raw(r.read_i32(28)?);
        let latitude = Latitude::from_raw(r.read_i32(27)?);
        let dimension_to_bow = r.read_u16(9)?;
        let dimension_to_stern = r.read_u16(9)?;
        let dimension_to_port = r.read_u8(6)?;
        let dimension_to_starboard = r.read_u8(6)?;
        let epfd_type = EpfdType::from_raw(r.read_u8(4)?);
        let timestamp = Timestamp::from_raw(r.read_u8(6)?);
        let off_position = r.read_bool()?;
        r.skip(8)?; // regional reserved
        let raim = r.read_bool()?;
        let virtual_aid = r.read_bool()?;
        let assigned = r.read_bool()?;
        r.skip(1)?; // spare

        // optional name extension: any remaining whole six-bit characters
        let mut name = name;
        let extra_chars = r.remaining_bits() / 6;
        if extra_chars > 0 {
            let extension = r.read_sixbit_ascii::<14>(extra_chars.min(14))?;
            name = append_fixed_str(name, &extension);
        }

        Ok(Self {
            repeat_indicator,
            mmsi,
            aid_type,
            name,
            position_accuracy,
            longitude,
            latitude,
            dimension_to_bow,
            dimension_to_stern,
            dimension_to_port,
            dimension_to_starboard,
            epfd_type,
            timestamp,
            off_position,
            raim,
            virtual_aid,
            assigned,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(21, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(u64::from(self.aid_type), 5)?;
        let (base_name, extension) = split_name(self.name.as_str());
        w.write_sixbit_ascii(base_name, 20)?;
        w.write_bool(self.position_accuracy)?;
        w.write_signed(i64::from(self.longitude.raw()), 28)?;
        w.write_signed(i64::from(self.latitude.raw()), 27)?;
        w.write_bits(u64::from(self.dimension_to_bow), 9)?;
        w.write_bits(u64::from(self.dimension_to_stern), 9)?;
        w.write_bits(u64::from(self.dimension_to_port), 6)?;
        w.write_bits(u64::from(self.dimension_to_starboard), 6)?;
        w.write_bits(u64::from(self.epfd_type.to_raw()), 4)?;
        w.write_bits(u64::from(self.timestamp.to_raw()), 6)?;
        w.write_bool(self.off_position)?;
        w.write_bits(0, 8)?; // regional reserved
        w.write_bool(self.raim)?;
        w.write_bool(self.virtual_aid)?;
        w.write_bool(self.assigned)?;
        w.write_bits(0, 1)?; // spare

        if !extension.is_empty() {
            w.write_sixbit_ascii(extension, extension.len())?;
        }
        Ok(())
    }
}

/// Splits a decoded name into its fixed 20-character base and any characters
/// past that (carried by the optional name-extension field).
fn split_name(name: &str) -> (&str, &str) {
    match name.char_indices().nth(20) {
        Some((byte_idx, _)) => (&name[..byte_idx], &name[byte_idx..]),
        None => (name, ""),
    }
}

fn append_fixed_str(base: FixedStr<34>, extension: &FixedStr<14>) -> FixedStr<34> {
    let mut buf = [0u8; 34];
    let base_str = base.as_str();
    let ext_str = extension.as_str();
    let base_bytes = base_str.as_bytes();
    let ext_bytes = ext_str.as_bytes();
    buf[..base_bytes.len()].copy_from_slice(base_bytes);
    buf[base_bytes.len()..base_bytes.len() + ext_bytes.len()].copy_from_slice(ext_bytes);
    FixedStr::from_raw(buf, base_bytes.len() + ext_bytes.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string::test_padded;

    fn sample(name: &str) -> AidToNavigationReport {
        AidToNavigationReport {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(993_123_456),
            aid_type: 1,
            name: test_padded(name),
            position_accuracy: true,
            longitude: Longitude::from_raw(-44_100_000),
            latitude: Latitude::from_raw(24_600_000),
            dimension_to_bow: 0,
            dimension_to_stern: 0,
            dimension_to_port: 0,
            dimension_to_starboard: 0,
            epfd_type: EpfdType::Surveyed,
            timestamp: Timestamp::from_raw(30),
            off_position: false,
            raim: true,
            virtual_aid: true,
            assigned: false,
        }
    }

    fn round_trip(original: AidToNavigationReport) -> AidToNavigationReport {
        let mut buf = [0u8; 64];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 21);
        AidToNavigationReport::decode(&mut r).unwrap()
    }

    #[test]
    fn round_trips_without_name_extension() {
        let original = sample("SEA BUOY 5");
        assert_eq!(round_trip(original), original);
    }

    #[test]
    fn round_trips_with_name_extension() {
        // 20 'A's (the fixed base field) followed by 10 'B's (the extension field)
        let mut buf = [b'@'; 34];
        buf[..20].copy_from_slice(&[b'A'; 20]);
        buf[20..30].copy_from_slice(&[b'B'; 10]);
        let name = FixedStr::from_raw(buf, 30);

        let mut original = sample("placeholder");
        original.name = name;

        let decoded = round_trip(original);
        assert_eq!(decoded, original);
        assert_eq!(decoded.name.as_str().len(), 30);
    }
}
