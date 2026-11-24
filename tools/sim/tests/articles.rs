//! The K5 cohort on the real Articles content (`docs/04-placement-planner.md`,
//! «Сборка сессии в прототипе», «Проверка»): no plan or decision of our
//! session builder breaks a rule. Draft KCs are treated as live in this
//! fixture only. Needs `nickel` on PATH (the dev shell and `nix flake
//! check` provide it). The full report is `just cohort`.

use std::path::Path;

use refl_core::model::KnowledgeModel;
use refl_core::plan::{Budget, Params};
use refl_sim::cohort::{CohortSpec, Policy, cohort};
use refl_sim::live_fixture;

#[test]
fn articles_cohort_breaks_no_rule() {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let out = content_build::build(&content_build::Options::new(content)).expect("build");
    let bundle = live_fixture(out.bundle.expect("the content builds"));
    let model = KnowledgeModel::default();
    let params = Params::default();
    let spec = CohortSpec {
        learners: 6,
        days: 30,
        seed: 3,
        budget: Budget::TwentyFive,
    };
    let r = cohort(&model, &bundle, &params, spec, Policy::Ours);
    assert!(r.violations.is_empty(), "{:?}", r.violations);
    assert!(r.checks > 1000, "{}", r.checks);
    assert!(r.answers > 0);
}
