# Run from the dev shell: `nix develop -c just [recipe]`, or `just` inside `nix develop`.
# `just` with no recipe runs everything CI runs, incrementally in ./target.

default: all

# fmt check, clippy, all tests, wasm32 core build, content build
all: fmt-check lint test wasm content

# Format the code
fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

# All tests, including the database ones (throwaway PostgreSQL) and the TS bindings check
test:
    cargo test --locked --workspace --all-features

# The core must build for the browser (no_std + alloc, default features)
wasm:
    cargo build --locked --release -p eng-core --target wasm32-unknown-unknown

# Check content/ and write the bundle to target/bundle.json
content:
    cargo run --locked --quiet -p content-build -- content --out target/bundle.json

# Regenerate the committed TS types in web/src/generated/core/
bindings:
    UPDATE_TS_BINDINGS=1 cargo test --locked -p eng-core --all-features --test ts_bindings

# Full hermetic CI, the same as on every commit
ci:
    nix flake check
