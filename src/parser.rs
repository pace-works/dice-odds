// Parses dice notation like "2d6+3" or "1d20-1d4" into a small AST.
//
// Strict mode is the default: no whitespace, lowercase 'd' only, no
// leading zeros, and every term after the first needs an explicit sign.
// --lenient relaxes those surface rules but never relaxes the numeric
// limits below, which exist to keep the distribution computation cheap.

use std::fmt;

const MAX_TERMS: usize = 32;
const MAX_TOTAL_DICE: u32 = 500;
const MAX_SIDES: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Plus,
    Minus,
}

#[derive(Debug, Clone, Copy)]
pub enum TermKind {
    Dice { count: u32, sides: u32 },
    Flat(u32),
}

#[derive(Debug, Clone, Copy)]
pub struct Term {
    pub sign: Sign,
    pub kind: TermKind,
}

#[derive(Debug)]
pub struct Expr {
    pub terms: Vec<Term>,
}

#[derive(Debug)]
pub enum ParseError {
    Empty,
    Whitespace(usize),
    UppercaseD(usize),
    LeadingZero(String),
    MissingSign(usize),
    EmptyTerm(usize),
    BadNumber(String),
    MissingSides(String),
    ZeroCount(String),
    ZeroSides(String),
    TooManyTerms(usize),
    TooManyDice(u32),
    TooManySides(u32),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "empty expression"),
            ParseError::Whitespace(pos) => {
                write!(f, "unexpected whitespace at position {pos} (use --lenient to allow it)")
            }
            ParseError::UppercaseD(pos) => {
                write!(f, "uppercase 'D' at position {pos} (use --lenient to allow it)")
            }
            ParseError::LeadingZero(tok) => {
                write!(f, "leading zero in number '{tok}' (use --lenient to allow it)")
            }
            ParseError::MissingSign(pos) => {
                write!(f, "missing '+' or '-' before term at position {pos}")
            }
            ParseError::EmptyTerm(pos) => write!(f, "empty term at position {pos}"),
            ParseError::BadNumber(tok) => write!(f, "'{tok}' is not a valid number"),
            ParseError::MissingSides(tok) => write!(f, "'{tok}' is missing a side count after 'd'"),
            ParseError::ZeroCount(tok) => write!(f, "'{tok}' has a dice count of zero"),
            ParseError::ZeroSides(tok) => write!(f, "'{tok}' has a side count of zero"),
            ParseError::TooManyTerms(n) => {
                write!(f, "expression has {n} terms, the limit is {MAX_TERMS}")
            }
            ParseError::TooManyDice(n) => {
                write!(f, "expression rolls {n} dice total, the limit is {MAX_TOTAL_DICE}")
            }
            ParseError::TooManySides(n) => {
                write!(f, "a die with {n} sides exceeds the limit of {MAX_SIDES}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse(input: &str, lenient: bool) -> Result<Expr, ParseError> {
    if input.is_empty() {
        return Err(ParseError::Empty);
    }

    let cleaned = if lenient {
        input.chars().filter(|c| !c.is_whitespace()).collect::<String>()
    } else {
        if let Some(pos) = input.find(char::is_whitespace) {
            return Err(ParseError::Whitespace(pos));
        }
        input.to_string()
    };

    if cleaned.is_empty() {
        return Err(ParseError::Empty);
    }

    if !lenient {
        if let Some(pos) = cleaned.find('D') {
            return Err(ParseError::UppercaseD(pos));
        }
    }

    let bytes = cleaned.as_bytes();
    let mut terms = Vec::new();
    let mut i = 0usize;
    let mut first = true;

    while i < bytes.len() {
        let sign = match bytes[i] {
            b'+' => {
                i += 1;
                Sign::Plus
            }
            b'-' => {
                i += 1;
                Sign::Minus
            }
            _ => {
                if first {
                    Sign::Plus
                } else {
                    return Err(ParseError::MissingSign(i));
                }
            }
        };
        first = false;

        let start = i;
        while i < bytes.len() && bytes[i] != b'+' && bytes[i] != b'-' {
            i += 1;
        }
        let token = &cleaned[start..i];
        if token.is_empty() {
            return Err(ParseError::EmptyTerm(start));
        }

        terms.push(parse_term(token, sign, lenient)?);

        if terms.len() > MAX_TERMS {
            return Err(ParseError::TooManyTerms(terms.len()));
        }
    }

    let total_dice: u32 = terms
        .iter()
        .filter_map(|t| match t.kind {
            TermKind::Dice { count, .. } => Some(count),
            TermKind::Flat(_) => None,
        })
        .sum();
    if total_dice > MAX_TOTAL_DICE {
        return Err(ParseError::TooManyDice(total_dice));
    }

    Ok(Expr { terms })
}

fn parse_term(token: &str, sign: Sign, lenient: bool) -> Result<Term, ParseError> {
    let d_pos = if lenient {
        token.find(|c| c == 'd' || c == 'D')
    } else {
        token.find('d')
    };

    match d_pos {
        Some(pos) => {
            let count_str = &token[..pos];
            let sides_str = &token[pos + 1..];

            let count = if count_str.is_empty() {
                if lenient {
                    1
                } else {
                    return Err(ParseError::BadNumber(token.to_string()));
                }
            } else {
                parse_number(count_str, lenient)?
            };

            if sides_str.is_empty() {
                return Err(ParseError::MissingSides(token.to_string()));
            }
            let sides = parse_number(sides_str, lenient)?;

            if count == 0 {
                return Err(ParseError::ZeroCount(token.to_string()));
            }
            if sides == 0 {
                return Err(ParseError::ZeroSides(token.to_string()));
            }
            if sides > MAX_SIDES {
                return Err(ParseError::TooManySides(sides));
            }

            Ok(Term {
                sign,
                kind: TermKind::Dice { count, sides },
            })
        }
        None => {
            let value = parse_number(token, lenient)?;
            Ok(Term {
                sign,
                kind: TermKind::Flat(value),
            })
        }
    }
}

fn parse_number(s: &str, lenient: bool) -> Result<u32, ParseError> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ParseError::BadNumber(s.to_string()));
    }
    if !lenient && s.len() > 1 && s.starts_with('0') {
        return Err(ParseError::LeadingZero(s.to_string()));
    }
    s.parse::<u32>().map_err(|_| ParseError::BadNumber(s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dice_terms(expr: &Expr) -> Vec<(Sign, u32, u32)> {
        expr.terms
            .iter()
            .map(|t| match t.kind {
                TermKind::Dice { count, sides } => (t.sign, count, sides),
                TermKind::Flat(_) => panic!("expected a dice term"),
            })
            .collect()
    }

    #[test]
    fn parses_single_die() {
        let expr = parse("1d20", false).unwrap();
        assert_eq!(dice_terms(&expr), vec![(Sign::Plus, 1, 20)]);
    }

    #[test]
    fn parses_dice_plus_flat_modifier() {
        let expr = parse("2d6+3", false).unwrap();
        assert_eq!(expr.terms.len(), 2);
        match expr.terms[1].kind {
            TermKind::Flat(v) => assert_eq!(v, 3),
            _ => panic!("expected a flat term"),
        }
    }

    #[test]
    fn parses_mixed_signed_terms() {
        let expr = parse("2d6-1d4", false).unwrap();
        assert_eq!(
            dice_terms(&expr),
            vec![(Sign::Plus, 2, 6), (Sign::Minus, 1, 4)]
        );
    }

    #[test]
    fn empty_input_is_rejected() {
        assert!(matches!(parse("", false), Err(ParseError::Empty)));
    }

    #[test]
    fn strict_mode_rejects_whitespace() {
        assert!(matches!(
            parse("2d6 + 3", false),
            Err(ParseError::Whitespace(3))
        ));
    }

    #[test]
    fn lenient_mode_strips_whitespace() {
        let expr = parse("2d6 + 3", true).unwrap();
        assert_eq!(expr.terms.len(), 2);
    }

    #[test]
    fn strict_mode_rejects_uppercase_d() {
        assert!(matches!(
            parse("2D6", false),
            Err(ParseError::UppercaseD(1))
        ));
    }

    #[test]
    fn lenient_mode_accepts_uppercase_d() {
        let expr = parse("2D6", true).unwrap();
        assert_eq!(dice_terms(&expr), vec![(Sign::Plus, 2, 6)]);
    }

    #[test]
    fn strict_mode_rejects_leading_zero() {
        assert!(matches!(
            parse("02d6", false),
            Err(ParseError::LeadingZero(_))
        ));
    }

    #[test]
    fn lenient_mode_accepts_leading_zero() {
        let expr = parse("02d6", true).unwrap();
        assert_eq!(dice_terms(&expr), vec![(Sign::Plus, 2, 6)]);
    }

    #[test]
    fn strict_mode_rejects_omitted_count() {
        assert!(matches!(
            parse("d6", false),
            Err(ParseError::BadNumber(_))
        ));
    }

    #[test]
    fn lenient_mode_defaults_omitted_count_to_one() {
        let expr = parse("d6", true).unwrap();
        assert_eq!(dice_terms(&expr), vec![(Sign::Plus, 1, 6)]);
    }

    #[test]
    fn adjacent_terms_without_a_sign_merge_into_one_bad_token() {
        // There's no separator between "1d6" and "1d4" here, so the
        // tokenizer (which only splits on '+'/'-') treats the whole
        // thing as a single term and fails parsing its side count.
        assert!(matches!(
            parse("1d61d4", false),
            Err(ParseError::BadNumber(_))
        ));
    }

    #[test]
    fn rejects_trailing_operator() {
        assert!(matches!(parse("1d6+", false), Err(ParseError::EmptyTerm(4))));
    }

    #[test]
    fn rejects_missing_sides() {
        assert!(matches!(
            parse("2d", false),
            Err(ParseError::MissingSides(_))
        ));
    }

    #[test]
    fn rejects_zero_count() {
        assert!(matches!(parse("0d6", false), Err(ParseError::ZeroCount(_))));
    }

    #[test]
    fn rejects_zero_sides() {
        assert!(matches!(parse("1d0", false), Err(ParseError::ZeroSides(_))));
    }

    #[test]
    fn rejects_non_numeric_token() {
        assert!(matches!(parse("abc", false), Err(ParseError::BadNumber(_))));
    }

    #[test]
    fn rejects_sides_over_limit() {
        assert!(matches!(
            parse("1d1001", false),
            Err(ParseError::TooManySides(1001))
        ));
    }

    #[test]
    fn accepts_sides_at_limit() {
        assert!(parse("1d1000", false).is_ok());
    }

    #[test]
    fn rejects_total_dice_over_limit() {
        assert!(matches!(
            parse("501d6", false),
            Err(ParseError::TooManyDice(501))
        ));
    }

    #[test]
    fn accepts_total_dice_at_limit_across_terms() {
        assert!(parse("250d6+250d4", false).is_ok());
    }

    #[test]
    fn rejects_too_many_terms() {
        let expr = (0..33).map(|_| "1d6").collect::<Vec<_>>().join("+");
        assert!(matches!(
            parse(&expr, false),
            Err(ParseError::TooManyTerms(_))
        ));
    }

    #[test]
    fn accepts_terms_at_limit() {
        let expr = (0..32).map(|_| "1d6").collect::<Vec<_>>().join("+");
        assert!(parse(&expr, false).is_ok());
    }

    #[test]
    fn bare_number_is_a_flat_term() {
        let expr = parse("5", false).unwrap();
        match expr.terms[0].kind {
            TermKind::Flat(v) => assert_eq!(v, 5),
            _ => panic!("expected a flat term"),
        }
    }
}
