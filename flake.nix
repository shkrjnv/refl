{
  description = "eng-learn: English learning platform (Rust core + WASM, axum, Postgres, SolidJS, Nickel)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
      crane,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system nixpkgs.legacyPackages.${system});

      # Rust version, components and targets come from rust-toolchain.toml.
      # sha256 pins the toolchain manifest; update it together with the channel.
      rustToolchain =
        system:
        fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-MFHoWVcPZKjFrLnQnodCXbUsfZcofGfo9tV9fTTvklg=";
        };

      cargo =
        system: pkgs:
        let
          inherit (pkgs) lib;
          craneLib = (crane.mkLib pkgs).overrideToolchain (_: rustToolchain system);
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              (craneLib.fileset.commonCargoSources ./.)
              ./core/clippy.toml
              # Committed TS bindings: the ts_bindings test checks they are current.
              ./web/src/generated
            ];
          };
          commonArgs = {
            inherit src;
            pname = "eng-learn";
            version = "0.0.0";
            strictDeps = true;
            cargoExtraArgs = "--locked --workspace --all-features";
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        {
          fmt = craneLib.cargoFmt { inherit src; inherit (commonArgs) pname version; };
          clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- -D warnings";
            }
          );
          test = craneLib.cargoTest (commonArgs // { inherit cargoArtifacts; });
          # The core must build for the browser (no_std + alloc, default features).
          core-wasm32 = craneLib.mkCargoDerivation (
            commonArgs
            // {
              pnameSuffix = "-core-wasm32";
              cargoArtifacts = null;
              buildPhaseCargoCommand = "cargo build --locked --release -p eng-core --target wasm32-unknown-unknown";
              installPhaseCommand = "mkdir -p $out";
              doInstallCargoArtifacts = false;
            }
          );
        };
    in
    {
      devShells = forAllSystems (
        system: pkgs: {
          default = pkgs.mkShell {
            name = "eng-learn";
            packages = [
              (rustToolchain system)
              pkgs.sqlx-cli
              pkgs.postgresql
              pkgs.nickel
              pkgs.nodejs
              pkgs.pnpm
            ];
            # Stages 3-5 bring back: piper-tts (audio), languagetool and
            # hunspell + dictionaries (writing checks), ffmpeg/opus-tools (audio
            # pipeline).

            DATABASE_URL = "postgres://eng_learn@localhost/eng_learn";
          };
        }
      );

      # Local CI: `nix flake check` runs fmt, clippy (-D warnings), all tests
      # (including the TS bindings freshness check) and the wasm32 build of the core.
      checks = forAllSystems (
        system: pkgs:
        cargo system pkgs
        // {
          devShell = self.devShells.${system}.default;
        }
      );

      formatter = forAllSystems (system: pkgs: pkgs.nixfmt);
    };
}
