//! One module per ITU-R M.1371 message type (or tightly related group).

mod aid_to_navigation;
mod base_station;
mod binary_ack;
mod binary_addressed;
mod binary_broadcast;
mod binary_multi_slot;
mod binary_single_slot;
mod long_range;
mod position_report_a;
mod position_report_b;
mod position_report_b_ext;
mod safety_addressed;
mod safety_broadcast;
mod sar_aircraft;
mod static_data_report;
mod static_voyage;

pub use aid_to_navigation::AidToNavigationReport;
pub use base_station::BaseStationReport;
pub use binary_ack::{Ack, Acknowledge};
pub use binary_addressed::BinaryAddressedMessage;
pub use binary_broadcast::BinaryBroadcastMessage;
pub use binary_multi_slot::MultiSlotBinaryMessage;
pub use binary_single_slot::SingleSlotBinaryMessage;
pub use long_range::LongRangeBroadcast;
pub use position_report_a::PositionReportClassA;
pub use position_report_b::PositionReportClassB;
pub use position_report_b_ext::PositionReportClassBExtended;
pub use safety_addressed::SafetyRelatedAddressed;
pub use safety_broadcast::SafetyRelatedBroadcast;
pub use sar_aircraft::SarAircraftPositionReport;
pub use static_data_report::{StaticDataReport, StaticDataReportPartA, StaticDataReportPartB};
pub use static_voyage::StaticVoyageData;
