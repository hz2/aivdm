{
  description = "no_std AIS (ITU-R M.1371) message parser and encoder, with a C FFI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      crane,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rustToolchain = pkgs.rust-bin.stable."1.98.0".default.override {
          extensions = [
            "rust-src"
            "clippy"
            "rustfmt"
          ];
          targets = [ "thumbv7em-none-eabi" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # cleanCargoSource only keeps Rust/Cargo-relevant files; the FFI
        # crate also needs its checked-in header, cbindgen config, and C
        # smoke test present for the header-diff and ffi-smoke-test checks.
        src = pkgs.lib.fileset.toSource {
          root = ./.;
          fileset = pkgs.lib.fileset.unions [
            (craneLib.fileset.commonCargoSources ./.)
            ./crates/aivdm-ffi/include
            ./crates/aivdm-ffi/cbindgen.toml
            ./crates/aivdm-ffi/tests
          ];
        };

        commonArgs = {
          inherit src;
          strictDeps = true;
          pname = "aivdm-workspace";
          version = "0.1.0";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        aivdm-cli = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            pname = "aivdm-cli";
            cargoExtraArgs = "-p aivdm-cli";
          }
        );

        aivdm = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            pname = "aivdm";
            cargoExtraArgs = "-p aivdm";
            doCheck = false;
          }
        );

        aivdm-ffi = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            pname = "aivdm-ffi";
            cargoExtraArgs = "-p aivdm-ffi";
            doCheck = false;
            postInstall = ''
              mkdir -p $out/include
              cp crates/aivdm-ffi/include/aivdm.h $out/include/
            '';
          }
        );
      in
      {
        packages = {
          default = aivdm-cli;
          inherit aivdm-cli aivdm aivdm-ffi;
        };

        checks = {
          inherit aivdm-cli aivdm aivdm-ffi;

          tests = craneLib.cargoTest (commonArgs // { inherit cargoArtifacts; });

          clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--workspace --all-targets -- -D warnings";
            }
          );

          fmt = craneLib.cargoFmt { inherit src; };

          no-std-verify = craneLib.mkCargoDerivation (
            commonArgs
            // {
              inherit cargoArtifacts;
              pnameSuffix = "-nostd-verify";
              buildPhaseCargoCommand = "cargo build -p aivdm --no-default-features --target thumbv7em-none-eabi";
              installPhaseCommand = "mkdir -p $out";
            }
          );

          # Regenerates the C header with the pinned cbindgen version and
          # fails if it differs from the checked-in
          # crates/aivdm-ffi/include/aivdm.h, catching drift between the
          # Rust FFI surface and the committed header (mirrors rustls-ffi's
          # CI header-diff check).
          header-diff = craneLib.mkCargoDerivation (
            commonArgs
            // {
              inherit cargoArtifacts;
              pnameSuffix = "-header-diff";
              nativeBuildInputs = [ pkgs.rust-cbindgen ];
              buildPhaseCargoCommand = ''
                cbindgen --config crates/aivdm-ffi/cbindgen.toml --crate aivdm-ffi --output generated-aivdm.h
                diff -u crates/aivdm-ffi/include/aivdm.h generated-aivdm.h
              '';
              installPhaseCommand = "mkdir -p $out";
            }
          );

          # Builds the FFI staticlib, compiles crates/aivdm-ffi/tests/smoke.c
          # against the committed header and links it statically, then runs
          # it. Proves the header and the compiled library agree with each
          # other (ABI-compatible) and that decoding actually produces
          # correct field values from C, not just that it links.
          ffi-smoke-test = craneLib.mkCargoDerivation (
            commonArgs
            // {
              inherit cargoArtifacts;
              pnameSuffix = "-ffi-smoke-test";
              buildPhaseCargoCommand = ''
                cargo build -p aivdm-ffi --release
                $CC -Wall -Wextra -Werror -o ffi_smoke \
                  crates/aivdm-ffi/tests/smoke.c \
                  -I crates/aivdm-ffi/include \
                  target/release/libaivdm_ffi.a \
                  -lpthread -ldl -lm
                ./ffi_smoke
              '';
              installPhaseCommand = "mkdir -p $out";
            }
          );
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ aivdm-cli ];
          packages = [
            rustToolchain
            pkgs.rust-analyzer
            pkgs.cargo-outdated
            pkgs.cargo-machete
            pkgs.rust-cbindgen
            pkgs.cargo-c
          ];
        };
      }
    );
}
