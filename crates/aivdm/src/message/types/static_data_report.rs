//! Static Data Report — message type 24 (Parts A and B).

use crate::bits::{BitReader, BitWriter};
use crate::error::{BitError, MessageError};
use crate::message::common::Mmsi;
use crate::string::FixedStr;

/// Part A of a Static Data Report: the vessel name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticDataReportPartA {
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Vessel name.
    pub name: FixedStr<20>,
}

/// Part B of a Static Data Report: type, dimensions, and call sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticDataReportPartB {
    /// Source station MMSI.
    pub mmsi: Mmsi,
    /// Type of ship and cargo (raw code, ITU-R M.1371 Table 41).
    pub ship_type: u8,
    /// Vendor ID: in practice, transponders encode this as a plain
    /// manufacturer name string (e.g. `"COMAR"`) rather than the formally
    /// specified vendor-code/model/serial sub-fields, so this crate follows
    /// that real-world convention and exposes it as text.
    pub vendor_id: FixedStr<7>,
    /// Radio call sign.
    pub call_sign: FixedStr<7>,
    /// Distance from GPS antenna to the bow, in meters (or, for an auxiliary
    /// craft, the MMSI of the mothership in the lower 30 bits of this group).
    pub dimension_to_bow: u16,
    /// Distance from GPS antenna to the stern, in meters.
    pub dimension_to_stern: u16,
    /// Distance from GPS antenna to the port side, in meters.
    pub dimension_to_port: u8,
    /// Distance from GPS antenna to the starboard side, in meters.
    pub dimension_to_starboard: u8,
}

/// A Static Data Report (message type 24), either Part A or Part B.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StaticDataReport {
    /// Part A: vessel name.
    A(StaticDataReportPartA),
    /// Part B: type, dimensions, and call sign.
    B(StaticDataReportPartB),
}

impl StaticDataReport {
    pub(crate) fn decode(r: &mut BitReader<'_>) -> Result<Self, MessageError> {
        r.skip(2)?; // repeat indicator (not modeled separately; rare in practice for type 24)
        let mmsi = Mmsi::from_raw(r.read_u32(30)?);
        let part_number = r.read_u8(2)?;
        match part_number {
            0 => {
                let name = r.read_sixbit_ascii(20)?;
                Ok(Self::A(StaticDataReportPartA { mmsi, name }))
            }
            1 => {
                let ship_type = r.read_u8(8)?;
                let vendor_id = r.read_sixbit_ascii(7)?;
                let call_sign = r.read_sixbit_ascii(7)?;
                let dimension_to_bow = r.read_u16(9)?;
                let dimension_to_stern = r.read_u16(9)?;
                let dimension_to_port = r.read_u8(6)?;
                let dimension_to_starboard = r.read_u8(6)?;
                Ok(Self::B(StaticDataReportPartB {
                    mmsi,
                    ship_type,
                    vendor_id,
                    call_sign,
                    dimension_to_bow,
                    dimension_to_stern,
                    dimension_to_port,
                    dimension_to_starboard,
                }))
            }
            other => Err(MessageError::InvalidEnumValue {
                field: "static_data_report.part_number",
                value: u32::from(other),
            }),
        }
    }

    pub(crate) fn encode(&self, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        w.write_bits(24, 6)?;
        w.write_bits(0, 2)?; // repeat indicator
        match self {
            Self::A(a) => {
                w.write_bits(u64::from(a.mmsi.raw()), 30)?;
                w.write_bits(0, 2)?; // part number A
                w.write_sixbit_ascii(a.name.as_str(), 20)?;
            }
            Self::B(b) => {
                w.write_bits(u64::from(b.mmsi.raw()), 30)?;
                w.write_bits(1, 2)?; // part number B
                w.write_bits(u64::from(b.ship_type), 8)?;
                w.write_sixbit_ascii(b.vendor_id.as_str(), 7)?;
                w.write_sixbit_ascii(b.call_sign.as_str(), 7)?;
                w.write_bits(u64::from(b.dimension_to_bow), 9)?;
                w.write_bits(u64::from(b.dimension_to_stern), 9)?;
                w.write_bits(u64::from(b.dimension_to_port), 6)?;
                w.write_bits(u64::from(b.dimension_to_starboard), 6)?;
                w.write_bits(0, 6)?; // spare
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string::test_padded;

    fn round_trip(original: StaticDataReport) -> StaticDataReport {
        let mut buf = [0u8; 32];
        let mut w = BitWriter::new(&mut buf);
        original.encode(&mut w).unwrap();
        let (armored, fill_bits) = w.finish().unwrap();

        let mut r = BitReader::new(armored, fill_bits);
        assert_eq!(r.read_u8(6).unwrap(), 24);
        StaticDataReport::decode(&mut r).unwrap()
    }

    #[test]
    fn round_trips_part_a() {
        let original = StaticDataReport::A(StaticDataReportPartA {
            mmsi: Mmsi::from_raw(366_053_209),
            name: test_padded("EXAMPLE VESSEL"),
        });
        assert_eq!(round_trip(original), original);
    }

    #[test]
    fn round_trips_part_b() {
        let original = StaticDataReport::B(StaticDataReportPartB {
            mmsi: Mmsi::from_raw(366_053_209),
            ship_type: 70,
            vendor_id: test_padded("ABCDEFG"),
            call_sign: test_padded("WDA9674"),
            dimension_to_bow: 100,
            dimension_to_stern: 20,
            dimension_to_port: 10,
            dimension_to_starboard: 10,
        });
        assert_eq!(round_trip(original), original);
    }

    #[test]
    fn decodes_real_captured_part_b() {
        // !AIVDM,1,1,,B,H3pro:4q3?=1B0000000000P7220,0*59 (raishub, 1332550009)
        // independently verified by libais: mmsi=261011240, part_num=1,
        // type_and_cargo=57, vendor_id='COMAR@@', callsign='@@@@@@@',
        // dim_a=4, dim_b=7, dim_c=2, dim_d=2, repeat_indicator=0.
        let payload = b"H3pro:4q3?=1B0000000000P7220";
        let mut r = BitReader::new(payload, 0);
        assert_eq!(r.read_u8(6).unwrap(), 24);
        let msg = StaticDataReport::decode(&mut r).unwrap();

        let StaticDataReport::B(b) = msg else {
            panic!("expected part B");
        };
        assert_eq!(b.mmsi.raw(), 261_011_240);
        assert_eq!(b.ship_type, 57);
        assert_eq!(b.vendor_id.as_str(), "COMAR");
        assert_eq!(b.call_sign.as_str(), "");
        assert_eq!(b.dimension_to_bow, 4);
        assert_eq!(b.dimension_to_stern, 7);
        assert_eq!(b.dimension_to_port, 2);
        assert_eq!(b.dimension_to_starboard, 2);
    }
}
