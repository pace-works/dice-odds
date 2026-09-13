// Turns a parsed expression into an exact probability distribution by
// convolving one uniform distribution per die. This is exact arithmetic
// on floats, not a Monte Carlo estimate, which is the whole point of the
// tool: small dice pools are cheap enough to enumerate fully.

use crate::parser::{Expr, Sign, Term, TermKind};

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

fn term_distribution(term: &Term) -> Distribution {
    let magnitude = match term.kind {
        TermKind::Dice { count, sides } => dice_distribution(count, sides),
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
