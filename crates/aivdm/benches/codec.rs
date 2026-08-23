//! Benchmarks for the decode/encode/reassembly hot paths.
//!
//! Run with `cargo bench -p aivdm`.

use std::hint::black_box;

use aivdm::{
    Channel, FragmentAssembler, LineDecoder, Sentence, decode_line, decode_payload, encode_line,
    encode_payload,
};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

/// A representative sample of real, single-fragment captured sentences
/// (see `tests/real_world_corpus.rs`), spanning short and long payloads.
const LINES: &[(&str, &str)] = &[
    (
        "type1_position_report_a",
        "!AIVDM,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*5C",
    ),
    (
        "type4_base_station",
        "!AIVDM,1,1,,A,;3P<f6iuiq00aOUu8DOD@j100000,0*44",
    ),
    (
        "type9_sar_aircraft",
        "!AIVDM,1,1,,B,9oVAuAI5;rRRv2OqTi?1uoP?=a@1,0*74",
    ),
    (
        "type18_position_report_b",
        "!AIVDM,1,1,,B,B6:VU2P0<:;2r84N5obLOwR2P0S9,0*23",
    ),
];

fn decode_line_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_line");
    for &(name, line) in LINES {
        group.bench_with_input(BenchmarkId::from_parameter(name), line, |b, line| {
            b.iter(|| decode_line(black_box(line)).unwrap());
        });
    }
    group.finish();
}

fn decode_payload_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_payload");
    for &(name, line) in LINES {
        let sentence = Sentence::parse(line).unwrap();
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &sentence,
            |b, sentence| {
                b.iter(|| decode_payload(black_box(sentence.payload), sentence.fill_bits).unwrap());
            },
        );
    }
    group.finish();
}

fn encode_payload_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_payload");
    for &(name, line) in LINES {
        let msg = decode_line(line).unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(name), &msg, |b, msg| {
            let mut buf = [0u8; 128];
            b.iter(|| {
                let _ = encode_payload(black_box(msg), &mut buf).unwrap();
            });
        });
    }
    group.finish();
}

fn encode_line_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_line");
    for &(name, line) in LINES {
        let msg = decode_line(line).unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(name), &msg, |b, msg| {
            let mut buf = [0u8; 128];
            b.iter(|| {
                let _ = encode_line(black_box(msg), Channel::B, false, &mut buf).unwrap();
            });
        });
    }
    group.finish();
}

/// A real two-fragment message (see
/// `type12_addressed_safety_related_message_two_fragments` in
/// `tests/real_world_corpus.rs`), used to benchmark reassembly.
const FRAGMENT_1: &str =
    "!AIVDM,2,1,1,A,<02PeAPpIkF06B?=PB?31P3?>DB?<rP@<51C5P3?>D13DPB?31P3?>DB,0*13";
const FRAGMENT_2: &str = "!AIVDM,2,2,1,A,?<P?>PF86P381>>5<PoqP6?BP=1>41D?BIPB5@?BD@,4*66";

fn fragment_reassembly_bench(c: &mut Criterion) {
    let s1 = Sentence::parse(FRAGMENT_1).unwrap();
    let s2 = Sentence::parse(FRAGMENT_2).unwrap();
    let mut assembler = FragmentAssembler::<256>::new();

    // fragment_number == 1 always resets the assembler internally, so it's
    // safe to reuse the same instance across iterations.
    c.bench_function("fragment_reassembly/two_part", |b| {
        b.iter(|| {
            assembler.push(black_box(&s1)).unwrap();
            black_box(assembler.push(black_box(&s2)).unwrap().unwrap());
        });
    });
}

fn line_decoder_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("line_decoder");

    group.bench_function("single_fragment", |b| {
        let mut decoder = LineDecoder::<256>::new();
        let line = LINES[0].1;
        b.iter(|| decoder.feed(black_box(line)).unwrap());
    });

    group.bench_function("two_fragment", |b| {
        let mut decoder = LineDecoder::<256>::new();
        b.iter(|| {
            decoder.feed(black_box(FRAGMENT_1)).unwrap();
            black_box(decoder.feed(black_box(FRAGMENT_2)).unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    decode_line_bench,
    decode_payload_bench,
    encode_payload_bench,
    encode_line_bench,
    fragment_reassembly_bench,
    line_decoder_bench,
);
criterion_main!(benches);
