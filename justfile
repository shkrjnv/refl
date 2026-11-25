# Run from the dev shell: `nix develop -c just [recipe]`, or `just` inside `nix develop`.
# `just` with no recipe runs everything CI runs, incrementally in ./target.

default: all

# fmt check, clippy, all tests, wasm32 core build, native = WASM fold and plan hashes, content build
all: fmt-check lint test wasm wasm-hash content

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
    cargo build --locked --release -p refl-core --target wasm32-unknown-unknown

# Fold one generated log and plan three sessions on it natively and in wasm32 (wasmtime): the hashes must match
wasm-hash events="20000" seed="7":
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --locked --release --quiet -p refl-sim --bin fold-hash --bin plan-hash --target wasm32-wasip1
    native="$(cargo run --locked --release --quiet -p refl-sim --bin fold-hash -- {{events}} {{seed}})"
    wasm="$(wasmtime run target/wasm32-wasip1/release/fold-hash.wasm {{events}} {{seed}})"
    echo "native: $native"
    echo "wasm:   $wasm"
    test "$native" = "$wasm"
    native="$(cargo run --locked --release --quiet -p refl-sim --bin plan-hash)"
    wasm="$(wasmtime run target/wasm32-wasip1/release/plan-hash.wasm)"
    echo "native: $native"
    echo "wasm:   $wasm"
    test "$native" = "$wasm"

# Time the learner-model fold of 100 000 generated events (native, release)
fold-speed:
    cargo run --locked --release --quiet -p refl-sim --bin fold-hash -- --time 100000

# Time build_session and next_item, native and in wasm32 under wasmtime (release)
plan-speed:
    cargo build --locked --release --quiet -p refl-sim --bin plan-hash --target wasm32-wasip1
    cargo run --locked --release --quiet -p refl-sim --bin plan-hash -- --time
    wasmtime run target/wasm32-wasip1/release/plan-hash.wasm --time

# The K5 cohort on content/ (draft KCs as live): our sessions against «только FSRS»
cohort learners="200" days="30" seed="1": content
    cargo run --locked --release --quiet -p refl-sim --bin cohort -- target/bundle.json {{learners}} {{days}} {{seed}}

# K5 cohort benchmark v1 (tools/sim/bench/v1): builds and runs the code of the tag bench-v1 in a temporary git worktree, not the current tree, and diffs its report against expected.txt of the tag; always identical unless the tag or the toolchain changed
bench-v1:
    #!/usr/bin/env bash
    set -euo pipefail
    root="$PWD"
    wt="$(mktemp -d)"
    trap 'git -C "$root" worktree remove --force "$wt"' EXIT
    git worktree add --detach --quiet "$wt" bench-v1
    cd "$wt"
    CARGO_TARGET_DIR="$root/target/bench-v1" cargo run --locked --release --quiet -p refl-sim --bin cohort -- content/bench/v1/bundle.json $(cat tools/sim/bench/v1/args) | diff -u tools/sim/bench/v1/expected.txt -
    echo "benchmark v1: identical to tools/sim/bench/v1/expected.txt at the tag bench-v1"

# P1: ours and only FSRS on the frozen bundle v2 (tools/sim/bench/p1), 200 learners x 30 days; the cohort seed is the argument
p1 seed="1":
    cargo run --locked --release --quiet -p refl-sim --bin p1 -- content/bench/v2/bundle.json 200 30 {{seed}}

# M1: why the knowledge model lags (tools/sim/m1/REPORT.md); replays the bench v1 cohort through variants of the knowledge update, about 1 minute
m1:
    cargo run --locked --release --quiet -p refl-sim --bin m1 -- content/bench/v1/bundle.json 200 30 1 > tools/sim/m1/output.txt

# M2: model v1.1 against v1 on a fresh cohort (seed 2, same frozen bundle); the same report as m1, read its «v1 against v1.1» section
m2:
    mkdir -p tools/sim/m2
    cargo run --locked --release --quiet -p refl-sim --bin m1 -- content/bench/v1/bundle.json 200 30 2 > tools/sim/m2/output-seed2.txt

# Review queue (by language, path:line) and all tasks as a learner sees them, in target/review/
review:
    cargo run --locked --quiet -p content-build -- report --review --out target/review content

# Mark records reviewed: just mark-reviewed --by Hara --lang ru --all  (or record ids, a review_group)
mark-reviewed *args:
    cargo run --locked --quiet -p content-build -- mark-reviewed {{args}}

# What keeps each draft KC from live, most valuable first
missing:
    cargo run --locked --quiet -p content-build -- report --missing content

# Check content/ and write the bundle to target/bundle.json
content:
    cargo run --locked --quiet -p content-build -- content --out target/bundle.json

# Regenerate the committed TS types in web/src/generated/core/
bindings:
    UPDATE_TS_BINDINGS=1 cargo test --locked -p refl-core --all-features --test ts_bindings

# Full hermetic CI, the same as on every commit
ci:
    nix flake check
