//! Property tests: the two expression parsers (user-defined surfaces and
//! procedural height maps) must agree with an independent evaluator.
//!
//! Random expression trees are printed twice: once with the fewest
//! parentheses standard precedence allows (what a user would type), and once
//! fully parenthesised. The first goes through our parser, the second through
//! the `exmex` crate, so any precedence or associativity mistake in our parser
//! shows up as a different number.

use geodesic_wallpaper::surface::procedural::parse_expression;
use geodesic_wallpaper::surface::user_defined::{UserDefinedSurface, UserSurfaceConfig};
use proptest::prelude::*;

#[derive(Debug, Clone)]
enum E {
    Num(f64),
    X,
    Y,
    Neg(Box<E>),
    Bin(char, Box<E>, Box<E>),
    Pow(Box<E>, Box<E>),
    Func(&'static str, Box<E>),
}

const P_ADD: u8 = 1;
const P_MUL: u8 = 2;
const P_NEG: u8 = 3;
const P_POW: u8 = 4;
const P_ATOM: u8 = 5;

fn prec(e: &E) -> u8 {
    match e {
        E::Num(_) | E::X | E::Y | E::Func(..) => P_ATOM,
        E::Neg(_) => P_NEG,
        E::Bin('+' | '-', ..) => P_ADD,
        E::Bin(..) => P_MUL,
        E::Pow(..) => P_POW,
    }
}

/// Wrap `s` in parentheses when the child's precedence is below `min`.
fn wrap(child: &E, min: u8, pow_op: &str) -> String {
    let s = minimal(child, pow_op);
    if prec(child) < min {
        format!("({s})")
    } else {
        s
    }
}

/// Print with as few parentheses as standard precedence needs:
/// `^` binds tightest and is right-associative, unary minus sits below it,
/// then `* /`, then `+ -`, all left-associative.
fn minimal(e: &E, pow_op: &str) -> String {
    match e {
        E::Num(v) => format!("{v}"),
        E::X => "x".into(),
        E::Y => "y".into(),
        // `--x` is not accepted by every parser, so a negated negation (or
        // anything looser than a power) gets parentheses.
        E::Neg(a) => format!("-{}", wrap(a, P_POW, pow_op)),
        E::Bin(op, a, b) => {
            let (lp, rp) = match op {
                '+' => (P_ADD, P_ADD),
                '-' => (P_ADD, P_MUL),
                '*' => (P_MUL, P_NEG),
                _ => (P_MUL, P_POW), // '/': right side must not be a product
            };
            // A unary minus is fine on the right of `* /`, but not of `/` when
            // that would change grouping; P_POW above already forces parens.
            format!("{}{op}{}", wrap(a, lp, pow_op), wrap(b, rp, pow_op))
        }
        E::Pow(a, b) => format!(
            "{}{pow_op}{}",
            wrap(a, P_ATOM, pow_op),
            wrap(b, P_NEG, pow_op)
        ),
        E::Func(f, a) => format!("{f}({})", minimal(a, pow_op)),
    }
}

/// Fully parenthesised, with x and y replaced by their values.
fn full(e: &E, x: f64, y: f64) -> String {
    match e {
        E::Num(v) => format!("({v})"),
        E::X => format!("({x})"),
        E::Y => format!("({y})"),
        E::Neg(a) => format!("(0-{})", full(a, x, y)),
        E::Bin(op, a, b) => format!("({}{op}{})", full(a, x, y), full(b, x, y)),
        E::Pow(a, b) => format!("({}^{})", full(a, x, y), full(b, x, y)),
        E::Func(f, a) => format!("{f}({})", full(a, x, y)),
    }
}

fn expr() -> impl Strategy<Value = E> {
    let leaf = prop_oneof![
        prop::sample::select(vec![0.5, 1.0, 1.5, 2.0, 3.0]).prop_map(E::Num),
        Just(E::X),
        Just(E::Y),
    ];
    leaf.prop_recursive(5, 24, 2, |inner| {
        prop_oneof![
            inner.clone().prop_map(|a| E::Neg(Box::new(a))),
            (
                prop::sample::select(vec!['+', '-', '*', '/']),
                inner.clone(),
                inner.clone()
            )
                .prop_map(|(op, a, b)| E::Bin(op, Box::new(a), Box::new(b))),
            (inner.clone(), prop::sample::select(vec![2.0, 3.0, -1.0]))
                .prop_map(|(a, k)| E::Pow(Box::new(a), Box::new(E::Num(k)))),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| E::Pow(Box::new(a), Box::new(b))),
            (prop::sample::select(vec!["sin", "cos", "exp"]), inner)
                .prop_map(|(f, a)| E::Func(f, Box::new(a))),
        ]
    })
}

fn oracle(e: &E, x: f64, y: f64) -> Option<f64> {
    let v: f64 = exmex::eval_str(&full(e, x, y)).ok()?;
    (v.is_finite() && v.abs() < 1e6).then_some(v)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn user_defined_matches_oracle(
        e in expr(),
        x in -2.0f64..2.0,
        y in -2.0f64..2.0,
        double_star in any::<bool>(),
    ) {
        let src = minimal(&e, if double_star { "**" } else { "^" });
        let surf = UserDefinedSurface::new(UserSurfaceConfig {
            z_expr: src.clone(),
            z_scale: 1.0,
            ..UserSurfaceConfig::default()
        });
        prop_assert!(surf.is_ok(), "failed to parse {src}: {:?}", surf.err().map(|e| e.to_string()));
        if let Some(want) = oracle(&e, x, y) {
            let got = surf.unwrap().eval_at(x, y);
            prop_assert!(
                (got - want).abs() <= 1e-9 * want.abs().max(1.0),
                "{src} at ({x}, {y}): parser {got}, oracle {want}"
            );
        }
    }

    #[test]
    fn procedural_matches_oracle(
        e in expr(),
        x in -2.0f64..2.0,
        y in -2.0f64..2.0,
    ) {
        let src = minimal(&e, "^");
        let parsed = parse_expression(&src);
        prop_assert!(parsed.is_ok(), "failed to parse {src}: {:?}", parsed.err());
        // The procedural evaluator is f32, so compare with the oracle at the
        // same f32-rounded inputs and a loose tolerance, and skip values that
        // are large or ill-conditioned in f32.
        let (xf, yf) = (x as f32, y as f32);
        if let Some(want) = oracle(&e, xf as f64, yf as f64) {
            let got = parsed.unwrap().eval(xf, yf) as f64;
            if want.abs() < 1e3 && got.is_finite() {
                prop_assert!(
                    (got - want).abs() <= 2e-2 * want.abs().max(1.0),
                    "{src} at ({xf}, {yf}): parser {got}, oracle {want}"
                );
            }
        }
    }

    /// Arbitrary text never panics or overflows the stack: it parses or errors.
    #[test]
    fn parsers_never_panic(s in "\\PC{0,200}") {
        let _ = parse_expression(&s);
        let _ = UserDefinedSurface::new(UserSurfaceConfig {
            z_expr: s,
            ..UserSurfaceConfig::default()
        });
    }

    /// Same, for strings made only of expression characters (much more
    /// likely to reach deep parser states than arbitrary Unicode).
    #[test]
    fn parsers_never_panic_on_expression_soup(s in "[-+*/^()xy0-9.eE, sincoexpatn]{0,300}") {
        let _ = parse_expression(&s);
        let _ = UserDefinedSurface::new(UserSurfaceConfig {
            z_expr: s,
            ..UserSurfaceConfig::default()
        });
    }
}
