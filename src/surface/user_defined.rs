//! User-defined mathematical surfaces via config.toml expressions.
//!
//! Allows users to specify a surface by providing a mathematical expression
//! for `z` as a function of `(x, y)` in their `config.toml`:
//!
//! ```toml
//! [surface.custom]
//! z_expr = "sin(x) * cos(y) * exp(-0.1 * (x*x + y*y))"
//! x_range = [-5.0, 5.0]
//! y_range = [-5.0, 5.0]
//! ```
//!
//! The expression is parsed at startup into an AST and evaluated at runtime
//! without any external crate dependency — the parser is a hand-written
//! recursive descent parser supporting:
//!
//! - Arithmetic: `+`, `-`, `*`, `/`, unary `-`
//! - Exponentiation: `^` or `**`
//! - Parentheses
//! - Variables: `x`, `y`
//! - Constants: `pi`, `e`
//! - Functions: `sin`, `cos`, `tan`, `exp`, `ln`, `log`, `sqrt`, `abs`,
//!   `sinh`, `cosh`, `tanh`, `asin`, `acos`, `atan`, `atan2(y,x)`,
//!   `sign`, `floor`, `ceil`
//!
//! ## Surface protocol
//!
//! [`UserDefinedSurface`] implements the same interface as the built-in
//! surfaces: it provides a `sample(u, v)` method returning `[f32; 3]` in
//! world space, and `normal(u, v)` returning the surface normal via central
//! finite differences.
//!
//! ## Usage
//!
//! ```rust
//! use geodesic_wallpaper::surface::user_defined::{UserDefinedSurface, UserSurfaceConfig};
//!
//! let cfg = UserSurfaceConfig {
//!     z_expr: "sin(x) * cos(y)".to_string(),
//!     x_range: [-3.14, 3.14],
//!     y_range: [-3.14, 3.14],
//!     z_scale: 1.0,
//! };
//! let surface = UserDefinedSurface::new(cfg).unwrap();
//! let pos = surface.sample(0.5, 0.5);
//! assert!(pos[2].is_finite());
//! ```

use std::fmt;

// ── public types ──────────────────────────────────────────────────────────────

/// Configuration for a user-defined surface loaded from `config.toml`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserSurfaceConfig {
    /// Mathematical expression for `z(x, y)`.
    pub z_expr: String,
    /// Range `[x_min, x_max]` mapped from parametric `u ∈ [0, 1]`.
    pub x_range: [f64; 2],
    /// Range `[y_min, y_max]` mapped from parametric `v ∈ [0, 1]`.
    pub y_range: [f64; 2],
    /// Uniform scale applied to the `z` output.
    pub z_scale: f64,
}

impl Default for UserSurfaceConfig {
    fn default() -> Self {
        Self {
            z_expr: "sin(x) * cos(y) * exp(-0.1 * (x*x + y*y))".to_string(),
            x_range: [-5.0, 5.0],
            y_range: [-5.0, 5.0],
            z_scale: 1.0,
        }
    }
}

/// Error type for expression parsing failures.
#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error: {}", self.0)
    }
}

/// A surface defined by a user-provided mathematical expression `z(x, y)`.
pub struct UserDefinedSurface {
    cfg: UserSurfaceConfig,
    ast: Expr,
}

impl UserDefinedSurface {
    /// Parse the expression in `cfg.z_expr` and construct the surface.
    pub fn new(cfg: UserSurfaceConfig) -> Result<Self, ParseError> {
        let ast = parse_expr(&cfg.z_expr)?;
        Ok(Self { cfg, ast })
    }

    /// Sample the surface at parametric coordinates `(u, v) ∈ [0, 1]²`.
    ///
    /// Maps `u → x` over `x_range` and `v → y` over `y_range`, then
    /// evaluates `z(x, y)`.  Returns `[x, y, z]` in world space.
    pub fn sample(&self, u: f32, v: f32) -> [f32; 3] {
        let x = self.cfg.x_range[0] + u as f64 * (self.cfg.x_range[1] - self.cfg.x_range[0]);
        let y = self.cfg.y_range[0] + v as f64 * (self.cfg.y_range[1] - self.cfg.y_range[0]);
        let z = eval(&self.ast, x, y) * self.cfg.z_scale;
        [x as f32, y as f32, z as f32]
    }

    /// Evaluate the expression directly at world coordinates `(x, y)`.
    pub fn eval_at(&self, x: f64, y: f64) -> f64 {
        eval(&self.ast, x, y) * self.cfg.z_scale
    }

    /// Compute the surface normal at `(u, v)` via central finite differences.
    pub fn normal(&self, u: f32, v: f32) -> [f32; 3] {
        let eps = 1e-4_f32;
        let p = self.sample(u, v);
        let px = self.sample(u + eps, v);
        let py = self.sample(u, v + eps);
        let tx = [px[0] - p[0], px[1] - p[1], px[2] - p[2]];
        let ty = [py[0] - p[0], py[1] - p[1], py[2] - p[2]];
        // Cross product tx × ty.
        let nx = tx[1] * ty[2] - tx[2] * ty[1];
        let ny = tx[2] * ty[0] - tx[0] * ty[2];
        let nz = tx[0] * ty[1] - tx[1] * ty[0];
        let len = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-8);
        [nx / len, ny / len, nz / len]
    }

    /// The parsed expression as a debug string (for display in the TUI).
    pub fn expression(&self) -> &str {
        &self.cfg.z_expr
    }
}

// ── AST ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    X,
    Y,
    Pi,
    E,
    Neg(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Func1(String, Box<Expr>),
    Func2(String, Box<Expr>, Box<Expr>),
}

fn eval(e: &Expr, x: f64, y: f64) -> f64 {
    match e {
        Expr::Num(n) => *n,
        Expr::X => x,
        Expr::Y => y,
        Expr::Pi => std::f64::consts::PI,
        Expr::E => std::f64::consts::E,
        Expr::Neg(a) => -eval(a, x, y),
        Expr::Add(a, b) => eval(a, x, y) + eval(b, x, y),
        Expr::Sub(a, b) => eval(a, x, y) - eval(b, x, y),
        Expr::Mul(a, b) => eval(a, x, y) * eval(b, x, y),
        Expr::Div(a, b) => {
            let d = eval(b, x, y);
            if d.abs() < 1e-300 {
                f64::NAN
            } else {
                eval(a, x, y) / d
            }
        }
        Expr::Pow(a, b) => eval(a, x, y).powf(eval(b, x, y)),
        Expr::Func1(name, a) => {
            let v = eval(a, x, y);
            match name.as_str() {
                "sin" => v.sin(),
                "cos" => v.cos(),
                "tan" => v.tan(),
                "exp" => v.exp(),
                "ln" | "log" => v.ln(),
                "sqrt" => v.sqrt(),
                "abs" => v.abs(),
                "sinh" => v.sinh(),
                "cosh" => v.cosh(),
                "tanh" => v.tanh(),
                "asin" => v.asin(),
                "acos" => v.acos(),
                "atan" => v.atan(),
                "sign" => v.signum(),
                "floor" => v.floor(),
                "ceil" => v.ceil(),
                _ => f64::NAN,
            }
        }
        Expr::Func2(name, a, b) => {
            let va = eval(a, x, y);
            let vb = eval(b, x, y);
            match name.as_str() {
                "atan2" => va.atan2(vb),
                "pow" => va.powf(vb),
                _ => f64::NAN,
            }
        }
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Deepest nesting of parentheses, function calls and powers accepted.
/// Deeper input is rejected so a hostile config cannot overflow the stack.
const MAX_DEPTH: usize = 64;

/// One-argument functions understood by the parser.
const FUNCS1: &[&str] = &[
    "sin", "cos", "tan", "exp", "ln", "log", "sqrt", "abs", "sinh", "cosh", "tanh", "asin", "acos",
    "atan", "sign", "floor", "ceil",
];
/// Two-argument functions understood by the parser.
const FUNCS2: &[&str] = &["atan2", "pow"];

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            input: s.as_bytes(),
            pos: 0,
            depth: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn consume(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn expect(&mut self, ch: u8) -> Result<(), ParseError> {
        self.skip_ws();
        match self.peek() {
            Some(c) if c == ch => {
                self.pos += 1;
                Ok(())
            }
            other => Err(ParseError(format!(
                "expected '{}', got {:?}",
                ch as char,
                other.map(|c| c as char)
            ))),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_add()
    }

    /// Run `f` one nesting level deeper, failing past [`MAX_DEPTH`].
    fn nested<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        if self.depth >= MAX_DEPTH {
            return Err(ParseError(format!(
                "expression nested deeper than {MAX_DEPTH} levels"
            )));
        }
        self.depth += 1;
        let r = f(self);
        self.depth -= 1;
        r
    }

    fn parse_add(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_mul()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'+') => {
                    self.pos += 1;
                    left = Expr::Add(Box::new(left), Box::new(self.parse_mul()?));
                }
                Some(b'-') => {
                    self.pos += 1;
                    left = Expr::Sub(Box::new(left), Box::new(self.parse_mul()?));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'*') => {
                    // `**` (power) is consumed by `parse_pow`, so this is `*`.
                    self.pos += 1;
                    left = Expr::Mul(Box::new(left), Box::new(self.parse_unary()?));
                }
                Some(b'/') => {
                    self.pos += 1;
                    left = Expr::Div(Box::new(left), Box::new(self.parse_unary()?));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        self.skip_ws();
        if self.peek() == Some(b'-') {
            self.pos += 1;
            return Ok(Expr::Neg(Box::new(self.parse_pow()?)));
        }
        if self.peek() == Some(b'+') {
            self.pos += 1;
        }
        self.parse_pow()
    }

    /// `primary (('^' | '**') unary)?`, right-associative, so `2^3^2` and
    /// `2**3**2` are both 512 and `-x**2` is `-(x**2)`.
    fn parse_pow(&mut self) -> Result<Expr, ParseError> {
        let base = self.parse_primary()?;
        self.skip_ws();
        let op_len = match (self.peek(), self.input.get(self.pos + 1)) {
            (Some(b'^'), _) => 1,
            (Some(b'*'), Some(b'*')) => 2,
            _ => 0,
        };
        if op_len > 0 {
            self.pos += op_len;
            let exp = self.nested(|p| p.parse_unary())?;
            return Ok(Expr::Pow(Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        self.skip_ws();
        match self.peek() {
            Some(b'(') => {
                self.pos += 1;
                let e = self.nested(|p| p.parse_expr())?;
                self.expect(b')')?;
                Ok(e)
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => self.parse_number(),
            Some(c) if c.is_ascii_alphabetic() || c == b'_' => self.parse_ident_or_func(),
            other => Err(ParseError(format!(
                "unexpected token: {:?}",
                other.map(|c| c as char)
            ))),
        }
    }

    fn parse_number(&mut self) -> Result<Expr, ParseError> {
        let start = self.pos;
        while matches!(
            self.peek(),
            Some(b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
        ) {
            // Only allow + / - after e/E.
            let c = self.peek().unwrap();
            if (c == b'+' || c == b'-') && self.pos == start {
                break;
            }
            if (c == b'+' || c == b'-') {
                // Allowed only immediately after e/E.
                let prev = if self.pos > start {
                    self.input[self.pos - 1]
                } else {
                    0
                };
                if prev != b'e' && prev != b'E' {
                    break;
                }
            }
            self.pos += 1;
        }
        let s = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|e| ParseError(e.to_string()))?;
        s.parse::<f64>()
            .map(Expr::Num)
            .map_err(|e| ParseError(e.to_string()))
    }

    fn parse_ident_or_func(&mut self) -> Result<Expr, ParseError> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == b'_') {
            self.pos += 1;
        }
        let name = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|e| ParseError(e.to_string()))?;
        // Keywords.
        match name {
            "x" => return Ok(Expr::X),
            "y" => return Ok(Expr::Y),
            "pi" | "PI" => return Ok(Expr::Pi),
            "e" | "E" => return Ok(Expr::E),
            _ => {}
        }
        // Function call.
        self.skip_ws();
        if self.peek() == Some(b'(') {
            self.pos += 1;
            let arg1 = self.nested(|p| p.parse_expr())?;
            self.skip_ws();
            if self.peek() == Some(b',') {
                self.pos += 1;
                if !FUNCS2.contains(&name) {
                    return Err(ParseError(format!(
                        "unknown two-argument function: {name} (known: {})",
                        FUNCS2.join(", ")
                    )));
                }
                let arg2 = self.nested(|p| p.parse_expr())?;
                self.expect(b')')?;
                return Ok(Expr::Func2(
                    name.to_string(),
                    Box::new(arg1),
                    Box::new(arg2),
                ));
            }
            self.expect(b')')?;
            if !FUNCS1.contains(&name) {
                return Err(ParseError(format!(
                    "unknown function: {name} (known: {})",
                    FUNCS1.join(", ")
                )));
            }
            return Ok(Expr::Func1(name.to_string(), Box::new(arg1)));
        }
        Err(ParseError(format!("unknown identifier: {name}")))
    }
}

fn parse_expr(s: &str) -> Result<Expr, ParseError> {
    let mut p = Parser::new(s.trim());
    let expr = p.parse_expr()?;
    p.skip_ws();
    if p.pos != p.input.len() {
        return Err(ParseError(format!(
            "trailing input at position {}: '{}'",
            p.pos,
            std::str::from_utf8(&p.input[p.pos..]).unwrap_or("?")
        )));
    }
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(expr: &str, x: f64, y: f64) -> f64 {
        let s = UserDefinedSurface::new(UserSurfaceConfig {
            z_expr: expr.to_string(),
            z_scale: 1.0,
            ..UserSurfaceConfig::default()
        })
        .expect("parse should succeed");
        s.eval_at(x, y)
    }

    // `**` is a power operator and must bind tighter than `*` (it used to be
    // parsed at the `*` level, so 2*x**2 meant (2*x)**2).
    #[test]
    fn bug_double_star_precedence() {
        assert_eq!(z("2*x**2", 3.0, 0.0), 18.0);
        assert_eq!(z("2**3**2", 0.0, 0.0), 512.0);
        assert_eq!(z("-x**2", 3.0, 0.0), -9.0);
        assert_eq!(z("x^2*2", 3.0, 0.0), 18.0);
    }

    // Unknown function names are a parse error, not a silent NaN surface.
    #[test]
    fn bug_unknown_function_is_error() {
        let r = UserDefinedSurface::new(UserSurfaceConfig {
            z_expr: "sinn(x)".to_string(),
            ..UserSurfaceConfig::default()
        });
        assert!(r.is_err());
        let r = UserDefinedSurface::new(UserSurfaceConfig {
            z_expr: "atan3(x, y)".to_string(),
            ..UserSurfaceConfig::default()
        });
        assert!(r.is_err());
    }

    // Deeply nested input is rejected instead of overflowing the stack.
    #[test]
    fn bug_deep_nesting_is_error() {
        let deep = format!("{}x{}", "(".repeat(100_000), ")".repeat(100_000));
        let r = UserDefinedSurface::new(UserSurfaceConfig {
            z_expr: deep,
            ..UserSurfaceConfig::default()
        });
        assert!(r.is_err());
        let towers = vec!["x"; 50_000].join("^");
        assert!(UserDefinedSurface::new(UserSurfaceConfig {
            z_expr: towers,
            ..UserSurfaceConfig::default()
        })
        .is_err());
        // Ordinary nesting still works.
        assert_eq!(z("((((x))))", 2.0, 0.0), 2.0);
    }

    fn surface(expr: &str) -> UserDefinedSurface {
        UserDefinedSurface::new(UserSurfaceConfig {
            z_expr: expr.to_string(),
            ..UserSurfaceConfig::default()
        })
        .expect("parse should succeed")
    }

    #[test]
    fn constant_expression() {
        let s = surface("3.14");
        let p = s.sample(0.5, 0.5);
        assert!((p[2] - 3.14_f32).abs() < 1e-4);
    }

    #[test]
    fn x_variable() {
        let s = surface("x");
        // At u=0, x = x_min = -5.0.
        let p = s.sample(0.0, 0.0);
        assert!((p[0] - (-5.0_f32)).abs() < 1e-4);
        assert!((p[2] - (-5.0_f32)).abs() < 1e-4);
    }

    #[test]
    fn sin_cos_expression() {
        let s = surface("sin(x) * cos(y)");
        let p = s.sample(0.5, 0.5);
        assert!(p[2].is_finite());
    }

    #[test]
    fn default_expression_evaluates() {
        let cfg = UserSurfaceConfig::default();
        let s = UserDefinedSurface::new(cfg).unwrap();
        let p = s.sample(0.5, 0.5);
        assert!(p[2].is_finite());
    }

    #[test]
    fn negative_unary() {
        let s = surface("-x");
        let p_center = s.sample(0.5, 0.5);
        // At u=0.5, x=0, so z=0.
        assert!(p_center[2].abs() < 1e-3);
    }

    #[test]
    fn power_expression() {
        let s = surface("x^2 + y^2");
        let p = s.sample(0.5, 0.5);
        // At centre x=y=0, z=0.
        assert!(p[2].abs() < 1e-3);
    }

    #[test]
    fn exp_expression() {
        let s = surface("exp(-x*x)");
        let p = s.sample(0.5, 0.5); // x=0 → exp(0)=1
        assert!((p[2] - 1.0_f32).abs() < 1e-3);
    }

    #[test]
    fn normal_is_normalised() {
        let s = surface("sin(x) * cos(y)");
        let n = s.normal(0.5, 0.5);
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-3);
    }

    #[test]
    fn invalid_expression_returns_error() {
        let result = UserDefinedSurface::new(UserSurfaceConfig {
            z_expr: "sin(x".to_string(),
            ..UserSurfaceConfig::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn pi_constant() {
        let s = surface("pi");
        let p = s.sample(0.0, 0.0);
        assert!((p[2] as f64 - std::f64::consts::PI).abs() < 1e-4);
    }

    #[test]
    fn two_arg_atan2() {
        let s = surface("atan2(y, x)");
        let p = s.sample(0.5, 0.5); // x=0, y=0 → atan2(0,0)=0
        assert!(p[2].is_finite());
    }
}
