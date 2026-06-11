//! Math workspace: step-by-step derivation checking, backed by fasteval
//! (pure Rust, MIT).
//!
//! Validation is purely numeric: two lines are considered equivalent when
//! they agree at every deterministic sample point. Expressions are compared
//! directly; equations (`lhs = rhs`) are reduced to residuals `lhs - rhs`
//! and accepted when the residuals are proportional by one nonzero constant
//! (covers add/subtract/multiply/divide applied to both sides — scaling by
//! an expression containing a variable changes the solution set and fails
//! the constant-ratio test). Sampling can't *prove* equivalence, but a false
//! positive needs the two lines to coincide at every sample point, which is
//! astronomically unlikely for honest mistakes.

use fasteval::{Evaler, Parser, Slab};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub enum StepStatus {
    /// First line of a derivation — the starting point, nothing to check.
    Given,
    /// Agrees with the previous line at every sample point.
    Valid,
    /// Differs from the previous line at a sample point — the step is wrong.
    Invalid,
    /// Couldn't be checked (parse error, unknown function, ...).
    Error(String),
}

impl StepStatus {
    /// Whether this step can serve as the baseline the next entry is checked
    /// against. Broken or unparseable lines stay visible but don't become the
    /// new reference.
    pub fn is_good(&self) -> bool {
        matches!(self, StepStatus::Given | StepStatus::Valid)
    }
}

pub struct MathStep {
    pub text: String,
    pub status: StepStatus,
}

/// Names resolved by the eval namespace — never sampled as variables.
const RESERVED: &[&str] = &["sqrt", "exp", "ln", "pi", "e"];

const N_TRIALS: usize = 5;
const TOL: f64 = 1e-9;
const ZERO_EPS: f64 = 1e-10;

/// A workspace line: a bare expression, or an equation kept as its two sides
/// (already preprocessed and syntax-checked).
pub enum Line {
    Expr(String),
    Eq(String, String),
}

pub fn parse_line(s: &str) -> Result<Line, String> {
    match s.matches('=').count() {
        0 => {
            let e = preprocess(s);
            syntax_check(&e)?;
            Ok(Line::Expr(e))
        }
        1 => {
            let (l, r) = s.split_once('=').unwrap();
            let (l, r) = (preprocess(l), preprocess(r));
            syntax_check(&l)?;
            syntax_check(&r)?;
            Ok(Line::Eq(l, r))
        }
        _ => Err("only one '=' allowed per line".to_string()),
    }
}

/// Insert the `*` fasteval needs where math convention leaves it implicit:
/// `2x` → `2*x`, `2(x+1)` → `2*(x+1)`, `(x+1)(x-1)` → `(x+1)*(x-1)`.
/// Digit-then-`e`-then-digit is left alone so scientific notation (`2e3`)
/// survives.
fn preprocess(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + 8);
    for (i, &c) in chars.iter().enumerate() {
        out.push(c);
        let Some(&next) = chars.get(i + 1) else { continue };
        let digit_then_name = c.is_ascii_digit() && (next.is_ascii_alphabetic() || next == '(');
        let close_then_open =
            c == ')' && (next.is_ascii_alphanumeric() || next == '(');
        let scientific = c.is_ascii_digit()
            && (next == 'e' || next == 'E')
            && matches!(chars.get(i + 2), Some(d) if d.is_ascii_digit() || *d == '+' || *d == '-');
        if (digit_then_name || close_then_open) && !scientific {
            out.push('*');
        }
    }
    out
}

/// The eval namespace: math functions fasteval lacks, the constants, then
/// variable lookup. Returning None for anything else surfaces as an eval
/// error ("unknown function").
fn namespace<'a>(
    vals: &'a BTreeMap<String, f64>,
) -> impl FnMut(&str, Vec<f64>) -> Option<f64> + 'a {
    |name, args| match (name, args.len()) {
        ("pi", 0) => Some(std::f64::consts::PI),
        ("e", 0) => Some(std::f64::consts::E),
        ("sqrt", 1) => Some(args[0].sqrt()),
        ("exp", 1) => Some(args[0].exp()),
        ("ln", 1) => Some(args[0].ln()),
        (_, 0) => vals.get(name).copied(),
        _ => None,
    }
}

fn syntax_check(text: &str) -> Result<(), String> {
    let mut slab = Slab::new();
    Parser::new()
        .parse(text, &mut slab.ps)
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

/// Variables appearing in `text`: everything the parser treats as a name,
/// minus what the namespace resolves.
fn collect_vars(text: &str, dst: &mut BTreeSet<String>) -> Result<(), String> {
    let mut slab = Slab::new();
    let expr_i = Parser::new()
        .parse(text, &mut slab.ps)
        .map_err(|e| format!("{e:?}"))?;
    for name in expr_i.from(&slab.ps).var_names(&slab) {
        if !RESERVED.contains(&name.as_str()) {
            dst.insert(name);
        }
    }
    Ok(())
}

fn eval_at(text: &str, vals: &BTreeMap<String, f64>) -> Result<f64, String> {
    let mut slab = Slab::new();
    let expr_i = Parser::new()
        .parse(text, &mut slab.ps)
        .map_err(|e| format!("{e:?}"))?;
    let mut ns = namespace(vals);
    expr_i
        .from(&slab.ps)
        .eval(&slab, &mut ns)
        .map_err(|e| format!("{e:?}"))
}

/// Deterministic pseudo-random sample in (0.3, 2.8) — positive and away from
/// 0/1 so ln, sqrt and division behave at most sample points.
fn sample_value(trial: usize, var_idx: usize) -> f64 {
    let h = (trial as u64 + 1)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((var_idx as u64 + 1).wrapping_mul(0xD1B5_4A32_D192_ED03));
    let frac = (h >> 11) as f64 / (1u64 << 53) as f64;
    0.3 + 2.5 * frac
}

fn sample_map(vars: &BTreeSet<String>, trial: usize) -> BTreeMap<String, f64> {
    vars.iter()
        .enumerate()
        .map(|(i, v)| (v.clone(), sample_value(trial, i)))
        .collect()
}

/// Check that `cur` is a mathematically valid step from `prev`.
pub fn check_step(prev: &str, cur: &str) -> StepStatus {
    let a = match parse_line(prev) {
        Ok(a) => a,
        Err(e) => return StepStatus::Error(format!("previous line no longer parses: {e}")),
    };
    let b = match parse_line(cur) {
        Ok(b) => b,
        Err(e) => return StepStatus::Error(format!("parse error: {e}")),
    };

    match (a, b) {
        (Line::Expr(a), Line::Expr(b)) => check_expr_equiv(&a, &b),
        (Line::Eq(l1, r1), Line::Eq(l2, r2)) => check_eq_equiv(&l1, &r1, &l2, &r2),
        (Line::Expr(_), Line::Eq(..)) => StepStatus::Error(
            "previous line is an expression, this one is an equation — can't compare".to_string(),
        ),
        (Line::Eq(..), Line::Expr(_)) => StepStatus::Error(
            "previous line is an equation — write this line as lhs = rhs too".to_string(),
        ),
    }
}

fn check_expr_equiv(a: &str, b: &str) -> StepStatus {
    let mut vars = BTreeSet::new();
    if let Err(e) = collect_vars(a, &mut vars).and_then(|_| collect_vars(b, &mut vars)) {
        return StepStatus::Error(e);
    }

    let mut compared = 0;
    for trial in 0..N_TRIALS {
        let vals = sample_map(&vars, trial);
        let (va, vb) = match (eval_at(a, &vals), eval_at(b, &vals)) {
            (Ok(va), Ok(vb)) => (va, vb),
            (Err(e), _) | (_, Err(e)) => return StepStatus::Error(e),
        };
        if !va.is_finite() || !vb.is_finite() {
            continue; // outside a function's domain at this point — try another
        }
        let scale = va.abs().max(vb.abs()).max(1.0);
        if (va - vb).abs() > TOL * scale {
            return StepStatus::Invalid;
        }
        compared += 1;
    }

    if compared == 0 {
        StepStatus::Error("could not evaluate at any sample point".to_string())
    } else {
        StepStatus::Valid
    }
}

fn check_eq_equiv(l1: &str, r1: &str, l2: &str, r2: &str) -> StepStatus {
    let mut vars = BTreeSet::new();
    for part in [l1, r1, l2, r2] {
        if let Err(e) = collect_vars(part, &mut vars) {
            return StepStatus::Error(e);
        }
    }

    let mut ratio: Option<f64> = None;
    let mut compared = 0;
    for trial in 0..N_TRIALS {
        let vals = sample_map(&vars, trial);
        let residual = |l: &str, r: &str| -> Result<f64, String> {
            Ok(eval_at(l, &vals)? - eval_at(r, &vals)?)
        };
        let (v1, v2) = match (residual(l1, r1), residual(l2, r2)) {
            (Ok(v1), Ok(v2)) => (v1, v2),
            (Err(e), _) | (_, Err(e)) => return StepStatus::Error(e),
        };
        if !v1.is_finite() || !v2.is_finite() {
            continue;
        }
        match (v1.abs() < ZERO_EPS, v2.abs() < ZERO_EPS) {
            (true, true) => continue, // shared root — uninformative point
            (true, false) | (false, true) => return StepStatus::Invalid,
            (false, false) => {}
        }
        let c = v1 / v2;
        if let Some(c0) = ratio {
            if (c - c0).abs() > TOL * c0.abs().max(1.0) {
                return StepStatus::Invalid;
            }
        } else {
            ratio = Some(c);
        }
        compared += 1;
    }

    // One usable sample can't distinguish a constant ratio from coincidence.
    if compared < 2 {
        StepStatus::Error("could not evaluate at enough sample points".to_string())
    } else {
        StepStatus::Valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expression_steps() {
        assert_eq!(check_step("(x+1)^2", "x^2 + 2*x + 1"), StepStatus::Valid);
        assert_eq!(check_step("(x+1)^2", "x^2 + 1"), StepStatus::Invalid);
        assert_eq!(check_step("sin(x)^2 + cos(x)^2", "1"), StepStatus::Valid);
        assert_eq!(check_step("(x^2-1)/(x+1)", "x - 1"), StepStatus::Valid);
        assert_eq!(check_step("(x+y)^2", "x^2 + 2*x*y + y^2"), StepStatus::Valid);
        // x-y vs y-x must NOT validate (they differ unless x == y)
        assert_eq!(check_step("x - y", "y - x"), StepStatus::Invalid);
    }

    #[test]
    fn custom_functions_and_constants() {
        assert_eq!(check_step("ln(exp(x))", "x"), StepStatus::Valid);
        assert_eq!(check_step("sqrt(x)^2", "x"), StepStatus::Valid);
        // pi must keep its real value
        assert_eq!(check_step("sin(pi*x)^2 + cos(pi*x)^2", "1"), StepStatus::Valid);
    }

    #[test]
    fn implicit_multiplication() {
        assert_eq!(preprocess("2x"), "2*x");
        assert_eq!(preprocess("2(x+1)"), "2*(x+1)");
        assert_eq!(preprocess("(x+1)(x-1)"), "(x+1)*(x-1)");
        assert_eq!(preprocess("2e3"), "2e3"); // scientific notation untouched
        assert_eq!(check_step("2x + 2x", "4x"), StepStatus::Valid);
        assert_eq!(check_step("(x+1)(x-1)", "x^2 - 1"), StepStatus::Valid);
    }

    #[test]
    fn equation_steps() {
        // subtract from both sides
        assert_eq!(check_step("2x + 4 = 6", "2x = 2"), StepStatus::Valid);
        // divide both sides by a constant
        assert_eq!(check_step("2x = 2", "x = 1"), StepStatus::Valid);
        // wrong solve
        assert_eq!(check_step("2x = 2", "x = 2"), StepStatus::Invalid);
        // rearrange across the equals sign
        assert_eq!(check_step("x^2 - 4 = 0", "x^2 = 4"), StepStatus::Valid);
        // multiplying both sides by x changes the solution set → not valid
        assert_eq!(check_step("x = 2", "x^2 = 2x"), StepStatus::Invalid);
    }

    #[test]
    fn errors_are_contained() {
        assert!(matches!(check_step("x + 1", "x +* 2"), StepStatus::Error(_)));
        // unknown function
        assert!(matches!(check_step("foo(x)", "x"), StepStatus::Error(_)));
        // mixing equation and expression lines
        assert!(matches!(check_step("2x = 2", "x"), StepStatus::Error(_)));
        assert!(matches!(check_step("x", "x = 1"), StepStatus::Error(_)));
        // double '='
        assert!(parse_line("x = 1 = 2").is_err());
    }
}
