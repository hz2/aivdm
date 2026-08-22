{
  description = "no_std AIS (ITU-R M.1371) message parser and encoder";

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
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;
          pname = "ais-dev";
          version = "0.1.0";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        ais-cli = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            pname = "ais-cli";
            cargoExtraArgs = "-p ais-cli";
          }
        );

        ais-core = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            pname = "ais-core";
            cargoExtraArgs = "-p ais-core";
            doCheck = false;
          }
        );
      in
      {
        packages = {
          default = ais-cli;
          inherit ais-cli ais-core;
        };

        checks = {
          inherit ais-cli ais-core;

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
              buildPhaseCargoCommand = "cargo build -p ais-core --no-default-features --target thumbv7em-none-eabi";
              installPhaseCommand = "mkdir -p $out";
            }
          );
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ ais-cli ];
          packages = [
            rustToolchain
            pkgs.rust-analyzer
            pkgs.cargo-outdated
            pkgs.cargo-machete
          ];
        };
      }
    );
}
