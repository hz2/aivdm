//! Channel Management — message type 22.

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::Mmsi;

/// Either the rectangular broadcast area or the two addressed destinations a
/// [`ChannelManagement`] command applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChannelManagementTarget {
    /// Applies to all stations within a rectangular area.
    Area {
        /// Longitude of the area's north-east corner, in 1/10-minute units.
        ne_longitude_raw: i32,
        /// Latitude of the area's north-east corner, in 1/10-minute units.
        ne_latitude_raw: i32,
        /// Longitude of the area's south-west corner, in 1/10-minute units.
        sw_longitude_raw: i32,
        /// Latitude of the area's south-west corner, in 1/10-minute units.
        sw_latitude_raw: i32,
    },
    /// Applies to up to two specifically addressed stations.
    Addressed {
        /// First addressed station MMSI.
        destination_mmsi_1: Mmsi,
        /// Second addressed station MMSI.
        destination_mmsi_2: Mmsi,
    },
}

/// Channel Management (message type 22, 168 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent single-bit wire field, not related state"
)]
pub struct ChannelManagement {
    /// How many times a repeater has relayed this message (0..=3).
    pub repeat_indicator: u8,
    /// Source station MMSI (a base station).
    pub mmsi: Mmsi,
    /// Channel A frequency (raw channel number).
    pub channel_a: u16,
    /// Channel B frequency (raw channel number).
    pub channel_b: u16,
    /// Transmit/receive mode to assign.
    pub tx_rx_mode: u8,
    /// Whether the assigned station(s) should use low transmit power.
    pub low_power: bool,
    /// The area or addressed destinations this command applies to.
    pub target: ChannelManagementTarget,
    /// Whether channel A uses the 12.5 kHz (`true`) or 25 kHz (`false`) bandwidth.
    pub channel_a_bandwidth: bool,
    /// Whether channel B uses the 12.5 kHz (`true`) or 25 kHz (`false`) bandwidth.
    pub channel_b_bandwidth: bool,
    /// Size of the transitional zone around the area, in nautical miles.
    pub zone_size: u8,
}

impl ChannelManagement {
    #[allow(
        clippy::similar_names,
        reason = "channel_a_bandwidth/channel_b_bandwidth mirror the spec's Channel A/B naming"
    )]
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        let repeat_indicator = r.read_u8(2)?;
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        r.skip(2)?; // spare
        let channel_a = r.read_u16(12)?;
        let channel_b = r.read_u16(12)?;
        let tx_rx_mode = r.read_u8(4)?;
        let low_power = r.read_bool()?;
        let addressed = r.read_bool()?;

        let target = if addressed {
            let destination_mmsi_1 = Mmsi::from_raw(r.read_u32(30)?);
            let destination_mmsi_2 = Mmsi::from_raw(r.read_u32(30)?);
            ChannelManagementTarget::Addressed {
                destination_mmsi_1,
                destination_mmsi_2,
            }
        } else {
            let ne_longitude_raw = r.read_i32(18)?;
            let ne_latitude_raw = r.read_i32(17)?;
            let sw_longitude_raw = r.read_i32(18)?;
            let sw_latitude_raw = r.read_i32(17)?;
            ChannelManagementTarget::Area {
                ne_longitude_raw,
                ne_latitude_raw,
                sw_longitude_raw,
                sw_latitude_raw,
            }
        };

        let channel_a_bandwidth = r.read_bool()?;
        let channel_b_bandwidth = r.read_bool()?;
        let zone_size = r.read_u8(3)?;
        // trailing spare: 23 bits (broadcast) or 33 bits (addressed), per the flag above
        #[allow(
            clippy::cast_possible_truncation,
            reason = "message payloads are at most ~1000 bits, always fits u32"
        )]
        let spare_bits = r.remaining_bits() as u32;
        r.skip(spare_bits)?;

        Ok(Self {
            repeat_indicator,
            mmsi,
            channel_a,
            channel_b,
            tx_rx_mode,
            low_power,
            target,
            channel_a_bandwidth,
            channel_b_bandwidth,
            zone_size,
        })
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(22, 6)?;
        w.write_bits(u64::from(self.repeat_indicator), 2)?;
        w.write_bits(u64::from(self.mmsi.raw()), 30)?;
        w.write_bits(0, 2)?; // spare
        w.write_bits(u64::from(self.channel_a), 12)?;
        w.write_bits(u64::from(self.channel_b), 12)?;
        w.write_bits(u64::from(self.tx_rx_mode), 4)?;
        w.write_bool(self.low_power)?;

        let spare_bits = match self.target {
            ChannelManagementTarget::Addressed {
                destination_mmsi_1,
                destination_mmsi_2,
            } => {
                w.write_bool(true)?;
                w.write_bits(u64::from(destination_mmsi_1.raw()), 30)?;
                w.write_bits(u64::from(destination_mmsi_2.raw()), 30)?;
                33
            }
            ChannelManagementTarget::Area {
                ne_longitude_raw,
                ne_latitude_raw,
                sw_longitude_raw,
                sw_latitude_raw,
            } => {
                w.write_bool(false)?;
                w.write_signed(i64::from(ne_longitude_raw), 18)?;
                w.write_signed(i64::from(ne_latitude_raw), 17)?;
                w.write_signed(i64::from(sw_longitude_raw), 18)?;
                w.write_signed(i64::from(sw_latitude_raw), 17)?;
                23
            }
        };

        w.write_bool(self.channel_a_bandwidth)?;
        w.write_bool(self.channel_b_bandwidth)?;
        w.write_bits(u64::from(self.zone_size), 3)?;
        w.write_bits(0, spare_bits)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(target: ChannelManagementTarget) -> ChannelManagement {
        ChannelManagement {
            repeat_indicator: 0,
            mmsi: Mmsi::from_raw(3_669_708),
            channel_a: 2087,
            channel_b: 2088,
            tx_rx_mode: 0,
            low_power: false,
            target,
            channel_a_bandwidth: false,
            channel_b_bandwidth: false,
            zone_size: 2,
        }
    }

    fn round_trip(original: ChannelManagement) -> ChannelManagement {
        let mut buf = [0u8; 32];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 22);
        ChannelManagement::decode(&mut r).unwrap()
    }

    #[test]
    fn round_trips_area_target() {
        let original = sample(ChannelManagementTarget::Area {
            ne_longitude_raw: 1000,
            ne_latitude_raw: 500,
            sw_longitude_raw: -1000,
            sw_latitude_raw: -500,
        });
        assert_eq!(round_trip(original), original);
    }

    #[test]
    fn round_trips_addressed_target() {
        let original = sample(ChannelManagementTarget::Addressed {
            destination_mmsi_1: Mmsi::from_raw(366_053_209),
            destination_mmsi_2: Mmsi::from_raw(366_999_999),
        });
        assert_eq!(round_trip(original), original);
    }
}
