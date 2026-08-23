//! Verification against a corpus of real, independently-decoded AIS
//! sentences drawn from `schwehr/libais`'s test suite
//! (<https://github.com/schwehr/libais/blob/main/test/test_data.py>), a
//! widely used, real-world-verified AIS decoder. Expected field values below
//! are taken from that corpus's checked results, not invented.
//!
//! These sentences also include the trailing `,<station>,<unix_ts>` metadata
//! that real-world aggregators (e.g. raishub) append after the checksum.

use aivdm::message::common::{EpfdType, NavigationStatus, Timestamp};
use aivdm::nmea::{FragmentAssembler, Sentence};
use aivdm::{AisMessage, decode_line, decode_payload};

fn decode(line: &str) -> AisMessage {
    decode_line(line).unwrap_or_else(|e| panic!("failed to decode {line:?}: {e}"))
}

#[test]
fn type4_base_station_report() {
    let msg = decode("!AIVDM,1,1,,A,;3P<f6iuiq00aOUu8DOD@j100000,0*44,raishub,1342569642");
    let AisMessage::BaseStationReport(m) = msg else {
        panic!("expected BaseStationReport")
    };
    assert_eq!(m.message_type, 11);
    assert_eq!(m.mmsi.raw(), 235_089_435);
    assert_eq!(m.utc_year, 2012);
    assert_eq!(m.utc_month, 7);
    assert_eq!(m.utc_day, 18);
    assert_eq!(m.utc_hour, 0);
    assert_eq!(m.utc_minute, 0);
    assert_eq!(m.utc_second, 41);
    assert!(!m.position_accuracy);
    assert!((m.longitude.as_degrees().unwrap() - (-5.689_583_333_333)).abs() < 1e-6);
    assert!((m.latitude.as_degrees().unwrap() - 54.729_72).abs() < 1e-6);
    assert_eq!(m.epfd_type, EpfdType::Gps);
    assert!(!m.raim);
}

#[test]
fn type6_binary_addressed_message() {
    let msg = decode("!AIVDM,1,1,,A,63m95T8uBK:0044@00P,2*7A");
    let AisMessage::BinaryAddressedMessage(m) = msg else {
        panic!("expected BinaryAddressedMessage")
    };
    assert_eq!(m.mmsi.raw(), 257_050_000);
    assert_eq!(m.sequence_number, 2);
    assert_eq!(m.destination_mmsi.raw(), 257_060_000);
    assert!(m.retransmit);
    assert_eq!(m.dac, 1);
    assert_eq!(m.fi, 1);

    let msg2 = decode("!AIVDM,1,1,,A,601uEQ8i02s:04<0@00000000000,0*12");
    let AisMessage::BinaryAddressedMessage(m2) = msg2 else {
        panic!("expected BinaryAddressedMessage")
    };
    assert_eq!(m2.mmsi.raw(), 2_053_508);
    assert_eq!(m2.destination_mmsi.raw(), 205_523_890);
    assert_eq!(m2.sequence_number, 2);
    assert!(!m2.retransmit);
    assert_eq!(m2.dac, 1);
    assert_eq!(m2.fi, 3);
}

#[test]
fn type7_binary_acknowledge() {
    let msg = decode("!AIVDM,1,1,,B,7l9B8LhP00PDLCvMdkg00?vD2D7w,0*3A,raishub,1342574351");
    let AisMessage::Acknowledge(m) = msg else {
        panic!("expected Acknowledge")
    };
    assert_eq!(m.message_type, 7);
    assert_eq!(m.mmsi.raw(), 278_169_715);
    assert_eq!(m.repeat_indicator, 3);
    let expected = [
        (134_218_245, 0),
        (474_998_636, 3),
        (250_609_727, 3),
        (620_908_671, 3),
    ];
    for (ack, (mmsi, seq)) in m.acks.into_iter().zip(expected) {
        let ack = ack.expect("expected all four acks present");
        assert_eq!(ack.mmsi.raw(), mmsi);
        assert_eq!(ack.sequence_number, seq);
    }
}

#[test]
fn type8_binary_broadcast_message() {
    let msg = decode(
        "!AIVDM,1,1,,B,804<o3@0Bj96WSWjHlPa321i=a58GwdtwwwwwwwwwwwwwwwwwwwwwCwwwt0,2*2F,raishub,1342574307",
    );
    let AisMessage::BinaryBroadcastMessage(m) = msg else {
        panic!("expected BinaryBroadcastMessage")
    };
    assert_eq!(m.mmsi.raw(), 4_405_005);
    assert_eq!(m.dac, 1);
    assert_eq!(m.fi, 11);
    assert_eq!(m.repeat_indicator, 0);
}

#[test]
fn type9_sar_aircraft_position_report() {
    let msg = decode("!AIVDM,1,1,,B,9oVAuAI5;rRRv2OqTi?1uoP?=a@1,0*74,raishub,1342572824");

    // the generic, message-type-agnostic AisMessage accessors should agree
    // with the type-specific fields checked below, against real data.
    assert_eq!(msg.mmsi().raw(), 509_902_149);
    assert_eq!(msg.repeat_indicator(), 3);
    let position = msg.position().unwrap();
    assert!((position.latitude - (-11.229_34)).abs() < 1e-6);
    assert!((position.longitude - 35.601_198_333_333).abs() < 1e-6);
    assert!((msg.sog_knots().unwrap() - 762.0).abs() < 1e-9);
    assert!((msg.cog_degrees().unwrap() - 50.3).abs() < 1e-6);

    let AisMessage::SarAircraftPositionReport(m) = msg else {
        panic!("expected SarAircraftPositionReport")
    };
    assert_eq!(m.mmsi.raw(), 509_902_149);
    assert_eq!(m.repeat_indicator, 3);
    assert_eq!(m.altitude_meters, 2324);
    assert_eq!(m.sog_knots, 762);
    assert!(m.position_accuracy);
    assert!((m.cog.degrees().unwrap() - 50.3).abs() < 1e-6);
    assert!((m.longitude.as_degrees().unwrap() - 35.601_198_333_333).abs() < 1e-6);
    assert!((m.latitude.as_degrees().unwrap() - (-11.229_34)).abs() < 1e-6);
    assert_eq!(m.timestamp, Timestamp::Second(30));
    assert!(m.dte_not_ready);
    assert!(m.assigned);
    assert!(m.raim);
}

#[test]
fn type10_utc_date_inquiry() {
    let msg = decode("!AIVDM,1,1,,A,:4`bLl0p3;Qd,0*77,raishub,1342569642");
    let AisMessage::UtcDateInquiry(m) = msg else {
        panic!("expected UtcDateInquiry")
    };
    assert_eq!(m.mmsi.raw(), 311_074_000);
    assert_eq!(m.destination_mmsi.raw(), 235_089_435);
    assert_eq!(m.repeat_indicator, 0);
}

#[test]
fn type12_addressed_safety_related_message_two_fragments() {
    let s1 = Sentence::parse(
        "!AIVDM,2,1,1,A,<02PeAPpIkF06B?=PB?31P3?>DB?<rP@<51C5P3?>D13DPB?31P3?>DB,0*13,raishub,1342580936",
    )
    .unwrap();
    let s2 = Sentence::parse(
        "!AIVDM,2,2,1,A,?<P?>PF86P381>>5<PoqP6?BP=1>41D?BIPB5@?BD@,4*66,raishub,1342580936",
    )
    .unwrap();

    let mut assembler = FragmentAssembler::<256>::new();
    assert!(assembler.push(&s1).unwrap().is_none());
    let complete = assembler
        .push(&s2)
        .unwrap()
        .expect("message should be complete");

    let msg = decode_payload(complete.armored, complete.fill_bits).unwrap();
    let AisMessage::SafetyRelatedAddressed(m) = msg else {
        panic!("expected SafetyRelatedAddressed")
    };
    assert_eq!(m.mmsi.raw(), 2_633_030);
    assert_eq!(m.destination_mmsi.raw(), 236_572_000);
    assert_eq!(m.sequence_number, 0);
    assert!(!m.retransmit);
    assert_eq!(
        m.text.as_str(),
        "FROM ROCA CONTROL: PLEASE CONTACT ROCA CONTROL ON VHF CHANNEL 79 FOR MANDATORY REPORT"
    );
}

#[test]
fn type13_safety_related_acknowledge() {
    let msg = decode("!AIVDM,1,1,,A,=3aDpM@pa=RmutjMeIojmgvLR0SE,0*28,raishub,1342582454");
    let AisMessage::Acknowledge(m) = msg else {
        panic!("expected Acknowledge")
    };
    assert_eq!(m.message_type, 13);
    assert_eq!(m.mmsi.raw(), 244_660_341);
    assert_eq!(m.repeat_indicator, 0);
    let expected = [
        (237_581_869, 1),
        (1_039_345_517, 1),
        (662_484_415, 3),
        (656_540_213, 1),
    ];
    for (ack, (mmsi, seq)) in m.acks.into_iter().zip(expected) {
        let ack = ack.expect("expected all four acks present");
        assert_eq!(ack.mmsi.raw(), mmsi);
        assert_eq!(ack.sequence_number, seq);
    }
}

#[test]
fn type14_safety_related_broadcast_message() {
    let msg = decode("!AIVDM,1,1,,A,>>M@rl1<59B1@E=@0000000,2*0D,raishub,1342621530");
    let AisMessage::SafetyRelatedBroadcast(m) = msg else {
        panic!("expected SafetyRelatedBroadcast")
    };
    assert_eq!(m.mmsi.raw(), 970_210_000);
    assert_eq!(m.repeat_indicator, 0);
    assert_eq!(m.text.as_str(), "SART TEST");
}

#[test]
fn type15_interrogation() {
    let msg = decode("!AIVDM,1,1,,B,?@0TcgRG`gmLD00000000000000,2*4F,raishub,1342570506");
    let AisMessage::Interrogation(m) = msg else {
        panic!("expected Interrogation")
    };
    assert_eq!(m.mmsi.raw(), 601_022);
    assert_eq!(m.repeat_indicator, 1);
    assert_eq!(m.station_1_mmsi.raw(), 636_010_327);
    assert_eq!(m.station_1_request_1.message_type, 5);
    assert_eq!(m.station_1_request_1.slot_offset, 0);
    let request_2 = m.station_1_request_2.expect("expected a second request");
    assert_eq!(request_2.message_type, 0);
    assert_eq!(request_2.slot_offset, 0);
    let station_2 = m.station_2.expect("expected a second station");
    assert_eq!(station_2.mmsi.raw(), 0);
    assert_eq!(station_2.request.message_type, 0);
    assert_eq!(station_2.request.slot_offset, 0);
}

#[test]
fn type16_assignment_mode_command() {
    let msg = decode("!AIVDM,1,1,,B,@bQBNdhP010Fh<LMb;:GLOvJP4@d,0*7F,raishub,1342577474");
    let AisMessage::AssignmentModeCommand(m) = msg else {
        panic!("expected AssignmentModeCommand")
    };
    assert_eq!(m.mmsi.raw(), 705_994_419);
    assert_eq!(m.repeat_indicator, 2);
    assert_eq!(m.first.destination_mmsi.raw(), 134_218_757);
    assert_eq!(m.first.offset, 2819);
    assert_eq!(m.first.increment, 113);
    let second = m.second.expect("expected a second destination");
    assert_eq!(second.destination_mmsi.raw(), 916_638_301);
    assert_eq!(second.offset, 3199);
    assert_eq!(second.increment, 922);
}

#[test]
fn type17_dgnss_broadcast_message() {
    let msg = decode("!AIVDM,1,1,,A,A6WWW6gP00a3PDlEKLrarOwUr8Mg,0*03,raishub,1342580511");
    let AisMessage::DgnssBroadcastMessage(m) = msg else {
        panic!("expected DgnssBroadcastMessage")
    };
    assert_eq!(m.mmsi.raw(), 444_196_634);
    assert_eq!(m.repeat_indicator, 0);
    assert!((f64::from(m.longitude_raw) / 600.0 - (-54.613_333_333_333)).abs() < 1e-6);
    assert!((f64::from(m.latitude_raw) / 600.0 - 35.033_333_333_333).abs() < 1e-6);
}

#[test]
fn type18_position_report_class_b() {
    let msg = decode("!AIVDM,1,1,,B,B6:VU2P0<:;2r84N5obLOwR2P0S9,0*23,raishub,1332581125");
    let AisMessage::PositionReportClassB(m) = msg else {
        panic!("expected PositionReportClassB")
    };
    assert_eq!(m.mmsi.raw(), 413_771_018);
    assert_eq!(m.repeat_indicator, 0);
    assert!((m.sog.knots().unwrap() - 4.8).abs() < 1e-6);
    assert!((m.cog.degrees().unwrap() - 250.3).abs() < 1e-6);
    assert!(m.position_accuracy);
    assert!(!m.raim);
    assert!(m.display_flag);
    assert!(!m.dsc_flag);
    assert!(m.band_flag);
    assert!(!m.message22_flag);
}

#[test]
fn type19_position_report_class_b_extended() {
    let msg = decode(
        "!AIVDM,1,1,,A,C7ldHCOH01nmtFP;UNuwQ6mTD2V30V:`B20000000000S0`WW320,0*27,raishub,1342581703",
    );
    let AisMessage::PositionReportClassBExtended(m) = msg else {
        panic!("expected PositionReportClassBExtended")
    };
    assert_eq!(m.mmsi.raw(), 525_015_117);
    assert_eq!(m.name.as_str(), "JASA SETIA");
    assert_eq!(m.ship_type, 70);
    assert_eq!(m.dimension_to_bow, 10);
    assert_eq!(m.dimension_to_stern, 79);
    assert_eq!(m.dimension_to_port, 14);
    assert_eq!(m.dimension_to_starboard, 6);
    assert!(!m.dte_not_ready);
    assert!(!m.raim);
    assert_eq!(m.timestamp, Timestamp::Second(43));
    assert_eq!(m.heading.degrees(), Some(141));
}

#[test]
fn type20_data_link_management_four_reservations() {
    let msg = decode("!AIVDM,1,1,,B,D09RFOhupNfq6DO6DgMJ>4giK6D,2*17,raishub,1351298504");
    let AisMessage::DataLinkManagement(m) = msg else {
        panic!("expected DataLinkManagement")
    };
    assert_eq!(m.mmsi.raw(), 9_999_999);
    let expected = [
        (990, 1, 7, 750),
        (1125, 1, 7, 1125),
        (759, 5, 5, 225),
        (764, 5, 5, 1125),
    ];
    for (reservation, (offset, num_slots, timeout, incr)) in
        m.reservations.into_iter().zip(expected)
    {
        let r = reservation.expect("expected all four reservations present");
        assert_eq!(r.offset, offset);
        assert_eq!(r.reserved_slots, num_slots);
        assert_eq!(r.timeout, timeout);
        assert_eq!(r.increment, incr);
    }
}

#[test]
fn type20_data_link_management_one_reservation() {
    let msg = decode("!AIVDM,1,1,,B,D02E35iqlg6D,0*41");
    let AisMessage::DataLinkManagement(m) = msg else {
        panic!("expected DataLinkManagement")
    };
    assert_eq!(m.mmsi.raw(), 2_442_007);
    let r = m.reservations[0].expect("expected one reservation");
    assert_eq!(r.offset, 1949);
    assert_eq!(r.reserved_slots, 2);
    assert_eq!(r.timeout, 7);
    assert_eq!(r.increment, 1125);
    assert!(m.reservations[1].is_none());
}

#[test]
fn type23_group_assignment_command() {
    let msg = decode("!AIVDM,1,1,,B,G02:KpP1R`sn@291njF00000900,2*1C,raishub,1335089672");
    let AisMessage::GroupAssignmentCommand(m) = msg else {
        panic!("expected GroupAssignmentCommand")
    };
    assert_eq!(m.mmsi.raw(), 2_268_130);
    assert_eq!(m.station_type, 6);
    assert_eq!(m.ship_type, 0);
    assert_eq!(m.tx_rx_mode, 0);
    assert_eq!(m.report_interval, 9);
    assert_eq!(m.quiet_time, 0);
    assert!((f64::from(m.ne_longitude_raw) / 600.0 - 2.63).abs() < 1e-6);
    assert!((f64::from(m.ne_latitude_raw) / 600.0 - 51.07).abs() < 1e-6);
    assert!((f64::from(m.sw_longitude_raw) / 600.0 - 1.826_666_666_666_7).abs() < 1e-6);
    assert!((f64::from(m.sw_latitude_raw) / 600.0 - 50.681_666_666_666_5).abs() < 1e-6);
}

#[test]
fn type25_single_slot_binary_message() {
    let msg = decode("!AIVDM,1,1,,B,ICa:3=`700>q6o;;fgBPqqwSP>1n,0*3D,raishub,1332550366");
    let AisMessage::SingleSlotBinaryMessage(m) = msg else {
        panic!("expected SingleSlotBinaryMessage")
    };
    assert_eq!(m.mmsi.raw(), 244_482_870);
    assert_eq!(m.repeat_indicator, 1);
    assert_eq!(m.destination_mmsi.unwrap().raw(), 29_360_366);

    let msg2 = decode("!AIVDM,1,1,,B,I6S`3Tg@T0a3REBEsjJcT?wSi0fM,0*02,raishub,1342654370");
    let AisMessage::SingleSlotBinaryMessage(m2) = msg2 else {
        panic!("expected SingleSlotBinaryMessage")
    };
    assert_eq!(m2.mmsi.raw(), 440_009_618);
    assert_eq!(m2.repeat_indicator, 0);
    assert_eq!(m2.destination_mmsi.unwrap().raw(), 874_775_184);
    let (dac, fi) = m2.app_id.expect("expected structured app id");
    assert_eq!(dac, 905);
    assert_eq!(fi, 21);
}

#[test]
fn type26_multi_slot_binary_message() {
    let msg = decode("!AIVDM,1,1,,B,J3`gb9@P8w8CC8TMeGBU<TH>0L@u,0*24,raishub,1342588508");
    let AisMessage::MultiSlotBinaryMessage(m) = msg else {
        panic!("expected MultiSlotBinaryMessage")
    };
    assert_eq!(m.mmsi.raw(), 244_050_469);
    assert_eq!(m.repeat_indicator, 0);
    assert!(!m.communication_state_itdma);
}

#[test]
fn type27_long_range_broadcast() {
    let msg = decode("!AIVDM,1,1,,A,KrJN9vb@0?wl20RH,0*7A,raishub,1342653118");
    let AisMessage::LongRangeBroadcast(m) = msg else {
        panic!("expected LongRangeBroadcast")
    };
    assert_eq!(m.mmsi.raw(), 698_845_690);
    assert_eq!(m.repeat_indicator, 3);
    assert_eq!(m.navigation_status, NavigationStatus::ReservedHsc);
    assert!(m.position_accuracy);
    assert!(!m.raim);
    assert_eq!(m.sog_knots, 1);
    assert_eq!(m.cog_degrees, 38);
    assert!((f64::from(m.longitude_raw) / 600.0 - 0.105).abs() < 1e-6);
    assert!((f64::from(m.latitude_raw) / 600.0 - (-2.553_333_333_333_3)).abs() < 1e-6);
}
