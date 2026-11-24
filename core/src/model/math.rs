//! Formulas of the knowledge model (`docs/03-layer2-learner-model.md`,
//! «Знание: вера о навыке»). Pure functions of numbers; every float function
//! goes through `libm`.

use alloc::vec::Vec;
use core::f64::consts::{PI, SQRT_2};

/// Logistic function, written so neither branch overflows.
pub fn sigmoid(x: f64) -> f64 {
    if x >= 0.0 {
        1.0 / (1.0 + libm::exp(-x))
    } else {
        let e = libm::exp(x);
        e / (1.0 + e)
    }
}

/// Standard normal CDF Φ.
pub fn phi(x: f64) -> f64 {
    0.5 * libm::erfc(-x / SQRT_2)
}

/// Slope flattened by uncertainty: `a / sqrt(1 + π a² σ² / 8)`.
pub fn effective_slope(a: f64, var: f64) -> f64 {
    a / libm::sqrt(1.0 + PI * a * a * var / 8.0)
}

/// One KC of a slot as the response model sees it: `P*_k = σ(a (θ − b))`
/// and the slope `a` it was computed with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KcTerm {
    pub p_star: f64,
    pub slope: f64,
}

impl KcTerm {
    pub fn new(theta: f64, b: f64, slope: f64) -> Self {
        Self {
            p_star: sigmoid(slope * (theta - b)),
            slope,
        }
    }
}

/// `p = g + (1 − g − s) ∏ P*_k`: conjunctive model with guessing and slip.
pub fn predict(g: f64, s: f64, terms: &[KcTerm]) -> f64 {
    g + (1.0 - g - s) * terms.iter().map(|t| t.p_star).product::<f64>()
}

/// Score and Fisher information of one KC for one observed answer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evidence {
    pub score: f64,
    pub info: f64,
}

/// One Newton step of the log-likelihood for every KC of the slot (Laplace
/// approximation): `p'_k = (1−g−s) a P*_k (1−P*_k) ∏_{j≠k} P*_j`,
/// `score_k = (y − p) p'_k / (p(1−p))`, `I_k = p'_k² / (p(1−p))`.
/// A degenerate `p(1−p)` (all mass at 0 or 1) carries no evidence.
pub fn evidence(g: f64, s: f64, y: bool, terms: &[KcTerm]) -> Vec<Evidence> {
    let p = predict(g, s, terms);
    let denom = p * (1.0 - p);
    let y = if y { 1.0 } else { 0.0 };
    terms
        .iter()
        .enumerate()
        .map(|(k, t)| {
            if denom <= 0.0 {
                return Evidence {
                    score: 0.0,
                    info: 0.0,
                };
            }
            let others: f64 = terms
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != k)
                .map(|(_, o)| o.p_star)
                .product();
            let dp = (1.0 - g - s) * t.slope * t.p_star * (1.0 - t.p_star) * others;
            Evidence {
                score: (y - p) * dp / denom,
                info: dp * dp / denom,
            }
        })
        .collect()
}

/// Posterior of one KC after evidence with weight `w`:
/// `σ²' = 1 / (1/σ² + w I)`, `μ' = μ + σ²' w score`.
pub fn posterior(mu: f64, var: f64, w: f64, e: Evidence) -> (f64, f64) {
    let var = 1.0 / (1.0 / var + w * e.info);
    (mu + var * w * e.score, var)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    /// The numeric example of docs/03 («Численный пример»): μ = 0, σ = 1,
    /// b = 0, a = 1, s = 0.05, one KC, no slope flattening.
    fn example(g: f64, y: bool) -> (f64, f64, f64) {
        let terms = [KcTerm::new(0.0, 0.0, 1.0)];
        let p = predict(g, 0.05, &terms);
        let e = evidence(g, 0.05, y, &terms)[0];
        let (mu, var) = posterior(0.0, 1.0, 1.0, e);
        (p, mu, libm::sqrt(var))
    }

    #[test]
    fn numeric_example_choice_of_four() {
        let (p, mu, sd) = example(0.25, true);
        assert!(close(p, 0.60, 1e-12));
        // score = 0.40 × 0.175 / 0.24 = 0.2917, I = 0.1276, σ²' = 0.8868.
        assert!(close(mu, 0.2587, 5e-4), "{mu}");
        assert!(close(sd, 0.94, 5e-3), "{sd}");
        let (_, mu, _) = example(0.25, false);
        assert!(close(mu, -0.39, 5e-3), "{mu}");
    }

    #[test]
    fn numeric_example_typed_input() {
        let (p, mu, sd) = example(0.0, true);
        assert!(close(p, 0.475, 1e-12));
        assert!(close(mu, 0.41, 5e-3), "{mu}");
        assert!(close(sd, 0.90, 5e-3), "{sd}");
        let (_, mu, _) = example(0.0, false);
        assert!(close(mu, -0.37, 5e-3), "{mu}");
    }

    #[test]
    fn typed_input_moves_belief_about_1_6_times_more_than_choice() {
        let ratio = example(0.0, true).1 / example(0.25, true).1;
        assert!(close(ratio, 1.6, 0.05), "{ratio}");
    }

    #[test]
    fn blame_goes_to_the_weaker_kc() {
        let terms = [KcTerm::new(2.0, 0.0, 1.0), KcTerm::new(-1.0, 0.0, 1.0)];
        let e = evidence(0.0, 0.05, false, &terms);
        assert!(e[1].score < e[0].score && e[0].score < 0.0);
    }

    #[test]
    fn slope_flattens_with_uncertainty() {
        assert!(close(effective_slope(1.0, 0.0), 1.0, 0.0));
        // σ = 1.5: 1 / sqrt(1 + π · 2.25 / 8)
        assert!(close(effective_slope(1.0, 2.25), 0.7286, 1e-4));
    }

    #[test]
    fn phi_is_the_normal_cdf() {
        assert!(close(phi(0.0), 0.5, 1e-15));
        assert!(close(phi(0.8416), 0.8, 1e-4));
        assert!(close(phi(-1.0), 0.158_655, 1e-6));
    }
}
