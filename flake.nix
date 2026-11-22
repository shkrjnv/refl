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
              ./tools/content-build/clippy.toml
              # content-build tests build the example content and the negative fixtures.
              ./content
              ./tools/content-build/tests/fixtures
              # sqlx::migrate! embeds the migrations at compile time.
              ./server/migrations
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
            # content-build runs `nickel export`; the server tests start a throwaway
            # PostgreSQL cluster (initdb in $TMPDIR), not a system service.
            nativeBuildInputs = [
              pkgs.nickel
              pkgs.postgresql
            ];
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          content-build = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              pname = "content-build";
              cargoExtraArgs = "--locked -p content-build";
              doCheck = false;
            }
          );
        in
        {
          fmt = craneLib.cargoFmt {
            inherit src;
            inherit (commonArgs) pname version;
          };
          clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- -D warnings";
            }
          );
          test = craneLib.cargoTest (commonArgs // { inherit cargoArtifacts; });
          # The real content must pass every check; the bundle and report are the output.
          content = pkgs.runCommand "eng-learn-content" { } ''
            mkdir -p $out
            ${content-build}/bin/content-build ${./content} \
              --nickel ${pkgs.nickel}/bin/nickel \
              --out $out/bundle.json > $out/report.txt \
              || { cat $out/report.txt; exit 1; }
          '';
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
              pkgs.just
            ];
            # Stages 3-5 bring back: piper-tts (audio), languagetool and
            # hunspell + dictionaries (writing checks), ffmpeg/opus-tools (audio
            # pipeline).

            DATABASE_URL = "postgres://eng_learn@localhost/eng_learn";
          };
        }
      );

      # Local CI: `nix flake check` runs fmt, clippy (-D warnings), all tests
      # (including the TS bindings freshness check), the wasm32 build of the core
      # and content-build on content/. Database tests run against a throwaway
      # PostgreSQL started inside the test check.
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
