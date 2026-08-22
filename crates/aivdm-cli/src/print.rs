//! Human-readable printing of decoded AIS messages.

use aivdm::AisMessage;

pub fn print_message(message: &AisMessage) {
    match message {
        AisMessage::PositionReportClassA(p) => {
            println!(
                "type={} mmsi={} nav_status={:?} sog={:?}kn cog={:?}deg lat={:?} lon={:?}",
                p.message_type,
                p.mmsi,
                p.navigation_status,
                p.sog.knots(),
                p.cog.degrees(),
                p.latitude.as_degrees(),
                p.longitude.as_degrees(),
            );
        }
        other => println!(
            "type={}: printing not yet implemented",
            other.message_type()
        ),
    }
}
