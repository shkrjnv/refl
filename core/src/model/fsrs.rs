//! FSRS-6 on `libm` (`docs/03-layer2-learner-model.md`, «Память и
//! забывание», «FSRS в ядре»). Only long-term reviews: the fold counts the
//! first answer per KC per day, so the same-day formula is not needed.

#![allow(clippy::many_single_char_names)] // names follow the FSRS formulas

use core::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::value::Timestamp;

/// Which reference the default weights are pinned to.
pub const DEFAULT_WEIGHTS_SOURCE: &str =
    "py-fsrs 6.3.3 DEFAULT_PARAMETERS = fsrs-rs 7.0.0 FSRS6_DEFAULT_PARAMETERS";

/// FSRS-6 default weights w₀…w₂₀; w₂₀ is the decay of the forgetting curve.
pub const DEFAULT_WEIGHTS: [f64; 21] = [
    0.212, 1.2931, 2.3065, 8.2956, 6.4133, 0.8334, 3.0194, 0.001, 1.8722, 0.1666, 0.796, 1.4835,
    0.0614, 0.2629, 1.6483, 0.6014, 1.8729, 0.5425, 0.0912, 0.0658, 0.1542,
];

/// Lower bound of stability in days, as in the reference.
pub const STABILITY_MIN: f64 = 0.001;
const DIFFICULTY_MIN: f64 = 1.0;
const DIFFICULTY_MAX: f64 = 10.0;

/// The grade the fold derives from an answer; the learner never sets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Grade {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

impl Grade {
    fn value(self) -> f64 {
        f64::from(self as u8)
    }
}

/// Memory of one KC.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Card {
    /// S in days: R falls to 0.9 after S days.
    pub stability: f64,
    /// D in `[1, 10]`.
    pub difficulty: f64,
    pub last_review: Timestamp,
}

/// `F = 0.9^(−1/w₂₀) − 1` for the default w₂₀, computed once. Every thread
/// computes the same bits, so a race only repeats the computation.
static DEFAULT_FACTOR_BITS: AtomicU64 = AtomicU64::new(0);

fn factor(decay: f64) -> f64 {
    let compute = || libm::pow(0.9, 1.0 / decay) - 1.0;
    if decay.to_bits() != (-DEFAULT_WEIGHTS[20]).to_bits() {
        return compute();
    }
    let cached = DEFAULT_FACTOR_BITS.load(Ordering::Relaxed);
    if cached != 0 {
        return f64::from_bits(cached);
    }
    let value = compute();
    DEFAULT_FACTOR_BITS.store(value.to_bits(), Ordering::Relaxed);
    value
}

/// `R(t) = (1 + F t / S)^(−w₂₀)`, `F = 0.9^(−1/w₂₀) − 1`; t in days.
pub fn retrievability(w: &[f64; 21], elapsed_days: f64, stability: f64) -> f64 {
    let decay = -w[20];
    let factor = factor(decay);
    libm::pow(
        1.0 + factor * elapsed_days.max(0.0) / stability.max(STABILITY_MIN),
        decay,
    )
}

fn clamp_difficulty(d: f64) -> f64 {
    d.clamp(DIFFICULTY_MIN, DIFFICULTY_MAX)
}

fn initial_difficulty(w: &[f64; 21], g: Grade) -> f64 {
    w[4] - libm::exp(w[5] * (g.value() - 1.0)) + 1.0
}

/// A new card after its first success.
pub fn start(w: &[f64; 21], g: Grade, at: Timestamp) -> Card {
    Card {
        stability: w[g as usize - 1].max(STABILITY_MIN),
        difficulty: clamp_difficulty(initial_difficulty(w, g)),
        last_review: at,
    }
}

fn next_difficulty(w: &[f64; 21], d: f64, g: Grade) -> f64 {
    let delta = -w[6] * (g.value() - 3.0);
    let damped = d + (10.0 - d) * delta / 9.0;
    // Mean reversion towards the initial difficulty of Easy (unclamped).
    clamp_difficulty(w[7] * initial_difficulty(w, Grade::Easy) + (1.0 - w[7]) * damped)
}

fn recall_stability(w: &[f64; 21], s: f64, d: f64, r: f64, g: Grade) -> f64 {
    let hard_penalty = if g == Grade::Hard { w[15] } else { 1.0 };
    let easy_bonus = if g == Grade::Easy { w[16] } else { 1.0 };
    s * (1.0
        + libm::exp(w[8])
            * (11.0 - d)
            * libm::pow(s, -w[9])
            * (libm::exp((1.0 - r) * w[10]) - 1.0)
            * hard_penalty
            * easy_bonus)
}

fn forget_stability(w: &[f64; 21], s: f64, d: f64, r: f64) -> f64 {
    let long_term = w[11]
        * libm::pow(d, -w[12])
        * (libm::pow(s + 1.0, w[13]) - 1.0)
        * libm::exp((1.0 - r) * w[14]);
    let short_term = s / libm::exp(w[17] * w[18]);
    long_term.min(short_term)
}

/// A review `elapsed_days` (whole days, ≥ 1) after the last one.
pub fn review(w: &[f64; 21], card: &Card, elapsed_days: f64, g: Grade, at: Timestamp) -> Card {
    let r = retrievability(w, elapsed_days, card.stability);
    let s = if g == Grade::Again {
        forget_stability(w, card.stability, card.difficulty, r)
    } else {
        recall_stability(w, card.stability, card.difficulty, r, g)
    };
    Card {
        stability: s.max(STABILITY_MIN),
        difficulty: next_difficulty(w, card.difficulty, g),
        last_review: at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: &[f64; 21] = &DEFAULT_WEIGHTS;
    const T0: Timestamp = Timestamp(0);

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    #[test]
    fn retrievability_is_0_9_at_t_equal_s_for_any_decay() {
        for decay in [0.1, 0.1542, 0.5, 0.8] {
            let mut w = DEFAULT_WEIGHTS;
            w[20] = decay;
            assert!(close(retrievability(&w, 7.0, 7.0), 0.9));
        }
        assert!(close(retrievability(W, 0.0, 3.0), 1.0));
    }

    #[test]
    fn first_success_takes_the_initial_weights() {
        let c = start(W, Grade::Good, T0);
        assert!(close(c.stability, 2.3065));
        // D₀(Good) = w₄ − e^{2 w₅} + 1 = 6.4133 − e^{1.6668} + 1
        assert!(close(c.difficulty, 6.4133 - libm::exp(1.6668) + 1.0));
        assert!(close(start(W, Grade::Easy, T0).difficulty, 1.0)); // clamped
    }

    #[test]
    fn good_review_by_hand() {
        let c = start(W, Grade::Good, T0);
        let (s, d) = (c.stability, c.difficulty);
        let r = retrievability(W, 3.0, s);
        let expected_s = s
            * (1.0
                + libm::exp(1.8722)
                    * (11.0 - d)
                    * libm::pow(s, -0.1666)
                    * (libm::exp((1.0 - r) * 0.796) - 1.0));
        let damped = d; // Good: Δ = 0
        let d_easy = 6.4133 - libm::exp(0.8334 * 3.0) + 1.0;
        let expected_d = 0.001 * d_easy + 0.999 * damped;
        let next = review(W, &c, 3.0, Grade::Good, Timestamp(1));
        assert!(close(next.stability, expected_s));
        assert!(close(next.difficulty, expected_d));
        assert!(next.stability > s);
    }

    #[test]
    fn lapse_takes_the_smaller_of_long_and_short_term() {
        let c = Card {
            stability: 30.0,
            difficulty: 5.0,
            last_review: T0,
        };
        let r = retrievability(W, 40.0, 30.0);
        let long = 1.4835
            * libm::pow(5.0, -0.0614)
            * (libm::pow(31.0, 0.2629) - 1.0)
            * libm::exp((1.0 - r) * 1.6483);
        let short = 30.0 / libm::exp(0.5425 * 0.0912);
        let next = review(W, &c, 40.0, Grade::Again, Timestamp(1));
        assert!(close(next.stability, long.min(short)));
        assert!(next.difficulty > 5.0);
    }

    #[test]
    fn grades_order_stability() {
        let c = start(W, Grade::Good, T0);
        let s: [f64; 4] = [Grade::Again, Grade::Hard, Grade::Good, Grade::Easy]
            .map(|g| review(W, &c, 5.0, g, Timestamp(1)).stability);
        assert!(s[0] < s[1] && s[1] < s[2] && s[2] < s[3], "{s:?}");
    }
}
