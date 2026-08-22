# aivdm

[![CI](https://github.com/hz2/aivdm/actions/workflows/ci.yml/badge.svg)](https://github.com/hz2/aivdm/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/aivdm.svg)](https://crates.io/crates/aivdm)
[![docs.rs](https://img.shields.io/docsrs/aivdm)](https://docs.rs/aivdm)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![no_std](https://img.shields.io/badge/no__std-yes-brightgreen.svg)](#)

A `no_std`, allocation-free parser and encoder for **AIS** (Automatic
Identification System) messages, per **ITU-R M.1371**, as carried over NMEA
0183 `!AIVDM`/`!AIVDO` sentences.

- **All 27 message types**, decode and encode, verified against a corpus of
  real, independently-decoded AIS captures (not just synthetic round trips).
- **`no_std`, no `alloc`**: works on bare-metal targets with no allocator.
  Multi-fragment sentence reassembly uses a fixed-capacity, caller-sized
  buffer instead of a growable one.
- **Zero mandatory dependencies.** The bit-level codec is hand-rolled: AIS's
  irregular field widths (6, 12, 22, 27, 28, 30, 38 bits, ...) and spec-fixed
  sentinel values don't map cleanly onto byte-aligned derive macros.
- **`#![forbid(unsafe_code)]`.** AIS messages are small (well under 1100
  bits); there's no meaningful performance case for unsafe here.
- **A C FFI** ([`aivdm-ffi`](crates/aivdm-ffi)) for calling the decoder from
  C or any language with a C FFI, with a CI-checked header and a linked,
  running smoke test.

## Crates

| Crate | Description |
| --- | --- |
| [`aivdm`](crates/aivdm) | The `no_std`, no-`alloc` parser/encoder library. |
| [`aivdm-cli`](crates/aivdm-cli) | A small command-line decoder (installs as `aivdm`). |
| [`aivdm-ffi`](crates/aivdm-ffi) | Decode-only C FFI bindings. |

## Quick start

```rust
let msg = aivdm::decode_line("!AIVDM,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*5C")?;

let aivdm::AisMessage::PositionReportClassA(report) = msg else {
    panic!("expected a position report");
};

println!("mmsi={} lat={:?} lon={:?}",
    report.mmsi,
    report.latitude.as_degrees(),
    report.longitude.as_degrees(),
);
```

`decode_line` handles single-fragment sentences, which cover the large
majority of AIS traffic. For multi-part messages, parse each line with
[`Sentence::parse`](https://docs.rs/aivdm/latest/aivdm/nmea/struct.Sentence.html)
and feed the fragments through a
[`FragmentAssembler`](https://docs.rs/aivdm/latest/aivdm/nmea/struct.FragmentAssembler.html)
(a fixed-capacity buffer sized by you, so it works without an allocator),
then decode the reassembled payload with `decode_payload`.

Encoding mirrors decoding:

```rust
let mut buf = [0u8; 64];
let (armored, fill_bits) = aivdm::encode_payload(&msg, &mut buf)?;
```

Add it to a project with:

```sh
cargo add aivdm
```

### From C

```c
#include "aivdm.h"

AivdmError err;
AivdmMessage *msg = aivdm_decode_line(line, &err);
if (msg != NULL) {
    double lat, lon;
    if (aivdm_message_position(msg, &lat, &lon)) {
        printf("mmsi=%u lat=%f lon=%f\n", aivdm_message_mmsi(msg), lat, lon);
    }
    aivdm_message_free(msg);
}
```

See [`crates/aivdm-ffi`](crates/aivdm-ffi) for the full API and
[`crates/aivdm-ffi/tests/smoke.c`](crates/aivdm-ffi/tests/smoke.c) for a
complete, working example. `cargo build -p aivdm-ffi` produces
`libaivdm_ffi.{a,so}` alongside the checked-in
`crates/aivdm-ffi/include/aivdm.h`; consumers who want a polished,
`pkg-config`-discoverable `libaivdm` can instead use
[`cargo-c`](https://github.com/lu-zero/cargo-c) (`cargo capi build`).

## Message type coverage

All 27 ITU-R M.1371 message types are implemented, decode and encode:

| Types | What |
| --- | --- |
| 1, 2, 3 | Position Report Class A |
| 4, 11 | Base Station Report / UTC and Date Response |
| 5 | Static and Voyage Related Data |
| 6, 8, 25, 26 | Binary Addressed/Broadcast Messages |
| 7, 13 | Binary / Safety Related Acknowledge |
| 9 | Standard SAR Aircraft Position Report |
| 10 | UTC and Date Inquiry |
| 12, 14 | Addressed / Broadcast Safety Related Message |
| 15 | Interrogation |
| 16 | Assignment Mode Command |
| 17 | DGNSS Broadcast Binary Message |
| 18, 19 | Standard / Extended Class B Position Report |
| 20 | Data Link Management Message |
| 21 | Aid-to-Navigation Report |
| 22 | Channel Management |
| 23 | Group Assignment Command |
| 24 | Static Data Report (Class B, Parts A/B) |
| 27 | Long Range AIS Broadcast |

Position reports, static/voyage data, base station reports, and the Class B
variants were checked against real captured sentences with hand-verified
field values. The rarer control/management types were additionally
cross-referenced against [`libais`](https://github.com/schwehr/libais)'s
well-tested decoder and its real-world test corpus — see
[`crates/aivdm/tests/real_world_corpus.rs`](crates/aivdm/tests/real_world_corpus.rs).

## Building

This project is built and checked with [Nix](https://nixos.org/):

```sh
nix develop        # dev shell: toolchain, rust-analyzer, cbindgen, cargo-c
nix flake check -L # fmt, clippy (pedantic), tests, no_std/no_alloc
                    # cross-compile verification, FFI header-diff, FFI smoke test
nix build           # aivdm-cli (default package)
nix build .#aivdm-ffi
```

CI runs the identical `nix flake check -L`, so there is one source of truth
for what "passing" means, locally and in GitHub Actions.

Without Nix, a standard `cargo build` / `cargo test` from the workspace root
also works with a recent stable Rust toolchain (edition 2024, MSRV tracked
in `Cargo.toml`).

## License

MIT. See [LICENSE](LICENSE).
