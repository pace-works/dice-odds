// Turns a parsed expression into an exact probability distribution by
// convolving one uniform distribution per die. This is exact arithmetic
// on floats, not a Monte Carlo estimate, which is the whole point of the
// tool: small dice pools are cheap enough to enumerate fully.

use crate::parser::{Expr, Keep, Sign, Term, TermKind};

pub struct Distribution {
    pub min: i64,
    pub probs: Vec<f64>,
}

impl Distribution {
    pub fn point(value: i64) -> Self {
        Distribution {
            min: value,
            probs: vec![1.0],
        }
    }

    pub fn max(&self) -> i64 {
        self.min + self.probs.len() as i64 - 1
    }

    pub fn mean(&self) -> f64 {
        self.probs
            .iter()
            .enumerate()
            .map(|(i, p)| (self.min + i as i64) as f64 * p)
            .sum()
    }

    pub fn prob_exact(&self, value: i64) -> f64 {
        if value < self.min || value > self.max() {
            0.0
        } else {
            self.probs[(value - self.min) as usize]
        }
    }

    pub fn prob_at_least(&self, value: i64) -> f64 {
        self.probs
            .iter()
            .enumerate()
            .filter(|(i, _)| self.min + *i as i64 >= value)
            .map(|(_, p)| p)
            .sum()
    }

    pub fn prob_at_most(&self, value: i64) -> f64 {
        self.probs
            .iter()
            .enumerate()
            .filter(|(i, _)| self.min + *i as i64 <= value)
            .map(|(_, p)| p)
            .sum()
    }
}

fn uniform_die(sides: u32) -> Distribution {
    let p = 1.0 / sides as f64;
    Distribution {
        min: 1,
        probs: vec![p; sides as usize],
    }
}

fn convolve(a: &Distribution, b: &Distribution) -> Distribution {
    let min = a.min + b.min;
    let mut probs = vec![0.0; a.probs.len() + b.probs.len() - 1];
    for (i, pa) in a.probs.iter().enumerate() {
        if *pa == 0.0 {
            continue;
        }
        for (j, pb) in b.probs.iter().enumerate() {
            probs[i + j] += pa * pb;
        }
    }
    Distribution { min, probs }
}

fn negate(d: &Distribution) -> Distribution {
    let mut probs = d.probs.clone();
    probs.reverse();
    Distribution {
        min: -d.max(),
        probs,
    }
}

fn dice_distribution(count: u32, sides: u32) -> Distribution {
    let die = uniform_die(sides);
    let mut acc = Distribution::point(0);
    for _ in 0..count {
        acc = convolve(&acc, &die);
    }
    acc
}

// Dice that are kept depend on each other, so plain convolution doesn't
// work. Instead walk the faces from the end that matters (highest face
// first for keep-highest, lowest first for keep-lowest) and decide how
// many of the dice show each face. state[j][sum] is the probability that
// j dice have been placed on faces so far and the kept ones total `sum`.
// Choosing k of the c-j unplaced dice for a face contributes
// C(c-j, k) * (1/sides)^k; the product over faces is the multinomial.
// That factor is built in log space because (1/sides)^k alone underflows
// for large pools.
fn keep_distribution(count: u32, sides: u32, keep: Keep) -> Distribution {
    let (kept, highest) = match keep {
        Keep::Highest(n) => (n as usize, true),
        Keep::Lowest(n) => (n as usize, false),
    };
    let c = count as usize;
    let s = sides as usize;
    let width = kept * s + 1;

    let mut ln_fact = vec![0.0f64; c + 1];
    for i in 1..=c {
        ln_fact[i] = ln_fact[i - 1] + (i as f64).ln();
    }
    let ln_sides = (s as f64).ln();

    let mut state = vec![vec![0.0f64; width]; c + 1];
    state[0][0] = 1.0;

    for step in 0..s {
        let face = if highest { s - step } else { step + 1 };
        let mut next = vec![vec![0.0f64; width]; c + 1];
        for j in 0..=c {
            let kept_before = j.min(kept);
            for k in 0..=(c - j) {
                let ln_w = ln_fact[c - j] - ln_fact[k] - ln_fact[c - j - k] - k as f64 * ln_sides;
                let w = ln_w.exp();
                let add = ((j + k).min(kept) - kept_before) * face;
                for sum in 0..width - add {
                    let p = state[j][sum];
                    if p != 0.0 {
                        next[j + k][sum + add] += p * w;
                    }
                }
            }
        }
        state = next;
    }

    // Every kept die shows at least 1, so sums below `kept` have no mass.
    let done = state.swap_remove(c);
    Distribution {
        min: kept as i64,
        probs: done[kept..].to_vec(),
    }
}

fn term_distribution(term: &Term) -> Distribution {
    let magnitude = match term.kind {
        TermKind::Dice {
            count,
            sides,
            keep: Some(keep),
        } => keep_distribution(count, sides, keep),
        TermKind::Dice { count, sides, keep: None } => dice_distribution(count, sides),
        TermKind::Flat(v) => Distribution::point(v as i64),
    };
    match term.sign {
        Sign::Plus => magnitude,
        Sign::Minus => negate(&magnitude),
    }
}

pub fn evaluate(expr: &Expr) -> Distribution {
    expr.terms
        .iter()
        .fold(Distribution::point(0), |acc, term| convolve(&acc, &term_distribution(term)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn eval(input: &str) -> Distribution {
        evaluate(&parse(input, false).unwrap())
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn two_d6_sums_correctly() {
        let d = eval("2d6");
        assert_eq!(d.min, 2);
        assert_eq!(d.max(), 12);
        assert!(close(d.prob_exact(7), 6.0 / 36.0));
    }

    #[test]
    fn subtraction_shifts_range_below_zero() {
        let d = eval("1d4-1d4");
        assert_eq!(d.min, -3);
        assert_eq!(d.max(), 3);
        assert!(close(d.prob_exact(0), 4.0 / 16.0));
    }

    #[test]
    fn keep_all_dice_matches_plain_sum() {
        let kept = eval("3d6kh3");
        let plain = eval("3d6");
        assert_eq!(kept.min, plain.min);
        for (a, b) in kept.probs.iter().zip(&plain.probs) {
            assert!(close(*a, *b));
        }
    }

    #[test]
    fn keep_highest_of_two_d20_is_advantage() {
        let d = eval("2d20kh1");
        assert_eq!(d.min, 1);
        assert_eq!(d.max(), 20);
        // P(max == v) = (2v - 1) / 400
        assert!(close(d.prob_exact(20), 39.0 / 400.0));
        assert!(close(d.prob_exact(1), 1.0 / 400.0));
    }

    #[test]
    fn keep_lowest_of_two_d20_is_disadvantage() {
        let d = eval("2d20kl1");
        // P(min == v) = (41 - 2v) / 400
        assert!(close(d.prob_exact(1), 39.0 / 400.0));
        assert!(close(d.prob_exact(20), 1.0 / 400.0));
    }

    #[test]
    fn four_d6_drop_lowest_known_values() {
        let d = eval("4d6kh3");
        assert_eq!(d.min, 3);
        assert_eq!(d.max(), 18);
        // 1296 outcomes: 18 needs at least three sixes (4 * 5 + 1 = 21
        // outcomes), 3 needs all ones.
        assert!(close(d.prob_exact(18), 21.0 / 1296.0));
        assert!(close(d.prob_exact(3), 1.0 / 1296.0));
        assert!(close(d.mean(), 12.2446952160494));
    }

    #[test]
    fn keep_distribution_sums_to_one() {
        let d = eval("10d20kl4");
        assert!(close(d.probs.iter().sum::<f64>(), 1.0));
    }

    #[test]
    fn keep_term_combines_with_modifier() {
        let d = eval("2d20kh1+5");
        assert_eq!(d.min, 6);
        assert_eq!(d.max(), 25);
    }
}
