//! Benchmark v1 of the K5 cohort (`docs/11-quality.md`, «Симулятор с
//! эталонами»; `bench/v1/README.md`): the frozen bundle, arguments and
//! seeds must give the recorded report byte for byte. v1 is never edited;
//! a simulator change goes into a v2 configuration.

use std::path::Path;
use std::process::Command;

#[test]
fn cohort_v1_output_is_unchanged() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bench = root.join("bench/v1");
    let args = std::fs::read_to_string(bench.join("args")).expect("bench/v1/args");
    let out = Command::new(env!("CARGO_BIN_EXE_cohort"))
        .arg(root.join("../../content/bench/v1/bundle.json"))
        .args(args.split_whitespace())
        .output()
        .expect("the cohort binary runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let got = String::from_utf8(out.stdout).expect("the report is UTF-8");
    let expected =
        std::fs::read_to_string(bench.join("expected.txt")).expect("bench/v1/expected.txt");
    if got != expected {
        let line = got
            .lines()
            .zip(expected.lines())
            .position(|(g, e)| g != e)
            .unwrap_or_else(|| got.lines().count().min(expected.lines().count()));
        panic!(
            "benchmark v1 output changed at line {}:\n  expected: {}\n  got:      {}\n(see tools/sim/bench/v1/README.md)",
            line + 1,
            expected.lines().nth(line).unwrap_or("<end>"),
            got.lines().nth(line).unwrap_or("<end>"),
        );
    }
}
