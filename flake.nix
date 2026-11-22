{
  description = "eng-learn: English learning platform (Rust core + WASM, axum, Postgres, SolidJS, Nickel)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, nixpkgs, fenix }:
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
            # pipeline), optional Haskell for content-build (docs/01-architecture.md).

            DATABASE_URL = "postgres://eng_learn@localhost/eng_learn";
          };
        }
      );

      # Local CI. Cargo checks (fmt, clippy, tests, wasm32 build) are added
      # together with the Cargo workspace in stage 0.
      checks = forAllSystems (
        system: pkgs: {
          devShell = self.devShells.${system}.default;
        }
      );

      formatter = forAllSystems (system: pkgs: pkgs.nixfmt);
    };
}
