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

# Web client tests and the production build
web:
    pnpm --dir web test
    pnpm --dir web build

# One-command local development: PostgreSQL on :5433, refl-server on :3000, Vite on :5173
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    export DATABASE_URL="postgres://refl@127.0.0.1:5433/refl"
    export BIND_ADDR="127.0.0.1:3000"
    export DEV_AUTH=1
    export REGISTRATION_MODE=closed
    if [ -f .env.local ]; then
        set -a
        # shellcheck disable=SC1091
        . ./.env.local
        set +a
    fi

    # Telegram needs the server token and, on the client, either the Login
    # Widget username or the numeric bot id. The id is the token prefix, so a
    # token alone is enough for the OAuth fallback.
    if [ -z "${VITE_TELEGRAM_BOT_ID:-}" ] && [[ "${TELEGRAM_BOT_TOKEN:-}" =~ ^([0-9]+): ]]; then
        export VITE_TELEGRAM_BOT_ID="${BASH_REMATCH[1]}"
    fi
    if [ -z "${VITE_TELEGRAM_BOT_USERNAME:-}" ] && [ -n "${TELEGRAM_BOT_USERNAME:-}" ]; then
        export VITE_TELEGRAM_BOT_USERNAME="${TELEGRAM_BOT_USERNAME}"
    fi
    if [ -z "${ADMIN_TELEGRAM_CHAT_ID:-}" ] && [ -n "${ADMIN_TELEGRAM_ID:-}" ]; then
        export ADMIN_TELEGRAM_CHAT_ID="${ADMIN_TELEGRAM_ID}"
    fi
    if [ -z "${TELEGRAM_BOT_TOKEN:-}" ]; then
        echo "refl dev: Telegram is disabled; set TELEGRAM_BOT_TOKEN, TELEGRAM_BOT_USERNAME and ADMIN_TELEGRAM_CHAT_ID in .env.local." >&2
    elif [ -z "${ADMIN_TELEGRAM_CHAT_ID:-}" ]; then
        echo "refl dev: TELEGRAM_BOT_TOKEN is set but ADMIN_TELEGRAM_CHAT_ID is missing; approval notifications and bot commands are disabled." >&2
    fi

    if [ ! -f .pgdata/PG_VERSION ]; then
        initdb -D .pgdata --username=refl --auth-local=trust --auth-host=trust --encoding=UTF8 >/dev/null
    fi
    if ! grep -Eq "^[[:space:]]*port[[:space:]]*=[[:space:]]*5433[[:space:]]*$" .pgdata/postgresql.conf; then
        printf "\nport = 5433\n" >> .pgdata/postgresql.conf
    fi
    if ! grep -Eq "^[[:space:]]*unix_socket_directories[[:space:]]*=[[:space:]]*'/tmp'[[:space:]]*$" .pgdata/postgresql.conf; then
        printf "unix_socket_directories = '/tmp'\n" >> .pgdata/postgresql.conf
    fi
    pg_ctl -D .pgdata status >/dev/null 2>&1 || pg_ctl -D .pgdata -l .pgdata/pg.log start

    ready=""
    for _ in $(seq 1 50); do
        if pg_isready -h /tmp -p 5433 -U refl -d postgres >/dev/null 2>&1; then
            ready=1
            break
        fi
        sleep 0.2
    done
    if [ -z "$ready" ]; then
        echo "PostgreSQL did not become ready on 127.0.0.1:5433" >&2
        exit 1
    fi
    if ! psql -h /tmp -p 5433 -U refl -d postgres -Atqc "select 1 from pg_database where datname = 'refl'" | grep -q 1; then
        createdb -h /tmp -p 5433 -U refl refl
    fi
    sqlx migrate run --source server/migrations --no-dotenv

    # A fresh database has no active bundle. Build and import the current
    # content pair before the server starts, so the first visit can fetch the
    # exact bundle + SCOWL dictionary the client verifies and caches.
    cargo run --locked --quiet -p content-build -- content --out target/bundle.json
    cargo run --locked --quiet -p refl-server -- import-bundle target/bundle.json

    server_pid=""
    web_pid=""
    cleanup() {
        trap - EXIT INT TERM
        for pid in "$web_pid" "$server_pid"; do
            if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
                kill -TERM -- "-$pid" 2>/dev/null || kill -TERM "$pid" 2>/dev/null || true
            fi
        done
        wait 2>/dev/null || true
    }
    trap cleanup EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM

    echo "refl dev: server http://127.0.0.1:3000, web http://127.0.0.1:5173"
    echo "Ctrl+C stops both processes and leaves PostgreSQL running."
    setsid bash -c 'exec cargo run --locked -p refl-server -- serve' &
    server_pid=$!
    setsid bash -c 'exec pnpm --dir web dev --host 0.0.0.0' &
    web_pid=$!

    status=0
    wait -n "$server_pid" "$web_pid" || status=$?
    exit "$status"

# The core and its wasm-bindgen bridge must build for the browser; the web
# client gets the generated JS glue, while compiled wasm/js stay untracked.
wasm:
    cargo build --locked --profile wasm-release -p refl-core --target wasm32-unknown-unknown
    cargo build --locked --profile wasm-release -p refl-core-wasm --target wasm32-unknown-unknown
    rm -rf web/src/generated/wasm
    mkdir -p web/src/generated/wasm
    wasm-bindgen --target web --out-dir web/src/generated/wasm target/wasm32-unknown-unknown/wasm-release/refl_core_wasm.wasm
    wasm-opt --strip-debug --strip-producers --strip-target-features --all-features -Oz \
        web/src/generated/wasm/refl_core_wasm_bg.wasm \
        -o web/src/generated/wasm/refl_core_wasm_bg.opt.wasm
    mv web/src/generated/wasm/refl_core_wasm_bg.opt.wasm \
        web/src/generated/wasm/refl_core_wasm_bg.wasm

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

# W24: 100k-event fold/replay/selection and native-vs-JSON-boundary benchmarks
stress:
    cargo test --locked --release -p refl-sim --test stress -- --ignored --nocapture --test-threads=1

# W24: the same fold in native release and wasm32-wasip1 under wasmtime; hashes must match
stress-wasm events="100000" seed="7":
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --locked --release --quiet -p refl-sim --bin stress --target wasm32-wasip1
    native="$(cargo run --locked --release --quiet -p refl-sim --bin stress -- fold {{events}} {{seed}})"
    wasm="$(wasmtime run target/wasm32-wasip1/release/stress.wasm fold {{events}} {{seed}})"
    echo "native: $native"
    echo "wasm:   $wasm"
    native_hash="$(printf '%s\n' "$native" | grep -o 'hash=[0-9a-f]*' | head -1)"
    wasm_hash="$(printf '%s\n' "$wasm" | grep -o 'hash=[0-9a-f]*' | head -1)"
    native_bytes="$(printf '%s\n' "$native" | grep -o 'snapshot_bytes=[0-9]*->[0-9]*' | head -1)"
    wasm_bytes="$(printf '%s\n' "$wasm" | grep -o 'snapshot_bytes=[0-9]*->[0-9]*' | head -1)"
    test -n "$native_hash"
    test "$native_hash" = "$wasm_hash"
    test "$native_bytes" = "$wasm_bytes"
    echo "cross-target hash and snapshot bytes match"

# Simulator v2: 1,000 learners, 600-step (~30-day) horizon plus the 200-step
# stress run, cluster bootstrap B=2000, with commit metadata in the report.
v2 learners="1000" seed="1": content
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p tools/sim/bench/v2
    tmp="$(mktemp)"
    trap 'rm -f "$tmp"' EXIT
    cargo run --locked --release --quiet -p refl-sim --bin v2 -- \
        content/bench/v3/bundle.json target/bundle.json {{learners}} {{seed}} > "$tmp"
    mv "$tmp" tools/sim/bench/v2/report-seed{{seed}}.txt
    trap - EXIT

# W24: 1,000 learners x 200 steps on the live and full 32-KC bundles (writes the report)
hetero learners="1000" steps="200" seed="1": content
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p tools/sim/bench/w24
    cargo run --locked --release --quiet -p refl-sim --bin hetero -- \
        content/bench/v3/bundle.json target/bundle.json {{learners}} {{steps}} {{seed}} \
        | tee tools/sim/bench/w24/hetero-seed{{seed}}.txt

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

# P1b: the p1 report plus same-day masteries and per-learner tables in target/p1b-seed<seed>-{ours,fsrs}.tsv (tools/sim/bench/p1b); the cohort seed is the argument
p1b seed="1":
    cargo run --locked --release --quiet -p refl-sim --bin p1b -- content/bench/v2/bundle.json 200 30 {{seed}} target/p1b-seed{{seed}}

# P1c: false mastery per KC and its evidence at mastery, on bundles v2 and v1 (tools/sim/bench/p1c); the cohort seed is the argument
p1c seed="1":
    cargo run --locked --release --quiet -p refl-sim --bin p1c -- content/bench/v2/bundle.json 200 30 {{seed}} > tools/sim/bench/p1c/bundle-v2-seed{{seed}}.txt
    cargo run --locked --release --quiet -p refl-sim --bin p1c -- content/bench/v1/bundle.json 200 30 {{seed}} > tools/sim/bench/p1c/bundle-v1-seed{{seed}}.txt

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

# Import an EPUB, text-layer PDF, TXT or Markdown into content/books/<slug>/source.md
book-import *args:
    cargo run --locked --quiet -p book-import -- {{args}}

# Compile indexed graded readers into target/books/ (after `just content`)
books: content
    cargo run --locked --quiet -p book-build -- content/books --bundle target/bundle.json --out target/books --web-out web/src/generated/books

# Explicitly accept a token-index change for published books (shifts book-mark offsets)
book-relock:
    cargo run --locked --quiet -p book-build -- content/books --bundle target/bundle.json --out target/books --web-out web/src/generated/books --relock

# Build the bundle + SCOWL dictionary and make that pair the active content.
# Idempotent; `just dev` runs it before starting the server so a fresh database
# can serve /v1/content/active and /v1/content/lexicon/{version}.
seed-content:
    #!/usr/bin/env bash
    set -euo pipefail
    export DATABASE_URL="${REFL_DATABASE_URL:-postgres://refl@127.0.0.1:5433/refl}"
    cargo run --locked --quiet -p content-build -- content --out target/bundle.json
    cargo run --locked --quiet -p refl-server -- import-bundle target/bundle.json

# Regenerate the committed TS types in web/src/generated/core/
bindings:
    UPDATE_TS_BINDINGS=1 cargo test --locked -p refl-core --all-features --test ts_bindings

# Full local CI: Rust/WASM, content, native = WASM hashes, and the web client
# (unit/component tests plus the production build). Run inside `nix develop`.
ci: fmt-check lint test wasm wasm-hash content web
