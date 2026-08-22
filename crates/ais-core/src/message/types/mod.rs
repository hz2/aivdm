//! One module per ITU-R M.1371 message type (or tightly related group).

mod aid_to_navigation;
mod base_station;
mod long_range;
mod position_report_a;
mod position_report_b;
mod position_report_b_ext;
mod sar_aircraft;
mod static_data_report;
mod static_voyage;

pub use aid_to_navigation::AidToNavigationReport;
pub use base_station::BaseStationReport;
pub use long_range::LongRangeBroadcast;
pub use position_report_a::PositionReportClassA;
pub use position_report_b::PositionReportClassB;
pub use position_report_b_ext::PositionReportClassBExtended;
pub use sar_aircraft::SarAircraftPositionReport;
pub use static_data_report::{StaticDataReport, StaticDataReportPartA, StaticDataReportPartB};
pub use static_voyage::StaticVoyageData;
