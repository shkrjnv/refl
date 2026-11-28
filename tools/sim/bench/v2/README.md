# Simulator v2

The v2 report is a separate, versioned output: `tools/sim/bench/v1/` and the
`bench-v1` tag are never edited (docs/11-quality.md, «Версии симулятора»).

- **Configuration:** `content/bench/v3/bundle.json` (live Articles) and
  `target/bundle.json` built by `just content` (32 learning KCs, drafts
  promoted only inside the fixture).
- **Cohort:** 1,000 learners, six archetypes, two L1 backgrounds, three
  strategies; the report runs both the 200-step stress horizon and a 600-step
  (~30-day) horizon.
- **Intervals:** learner-cluster bootstrap, SplitMix64, B = 2,000; 80% and
  95% percentile intervals. Fewer than six learners omit the interval.
- **Metadata:** every report names the git revision and whether the tree was
  dirty when it was generated.
- **Reproduce:** `just v2 1000 1` (writes `report-seed1.txt`).

The 200-step section is the W24 stress check; the 600-step section is the
realistic multi-day horizon. v1 remains reproducible only at its frozen tag.
