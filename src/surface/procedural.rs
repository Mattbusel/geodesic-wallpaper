//! Procedural surface generation.
//!
//! Generates surfaces of the form `z = f(x, y)` where `f` is defined by either
//! a built-in procedural function (Perlin noise, fractals) or a user-supplied
//! mathematical expression string parsed at runtime.
//!
//! # Expression syntax
//!
//! The expression parser supports:
//! - Variables: `x`, `y`
//! - Constants: numeric literals, `pi`, `e`
//! - Operators: `+`, `-`, `*`, `/`, `^` (power)
//! - Functions: `sin`, `cos`, `tan`, `sqrt`, `abs`, `exp`, `ln`
//!
//! # Examples
//!
//! ```rust
//! use geodesic_wallpaper::surface::procedural::{ProceduralSurface, ProceduralMode};
//!
//! // Built-in Perlin noise surface.
//! let surf = ProceduralSurface::perlin(2.0, 4, 1.0);
//!
//! // User expression: saddle-like ripple.
//! let expr = ProceduralSurface::from_expression("sin(x) * cos(y) * 0.5").unwrap();
//! ```

use glam::Vec3;
use rand::RngCore;

use crate::surface::Surface;

// ── Expression AST ────────────────────────────────────────────────────────────

/// AST node for the expression parser.
#[derive(Debug, Clone)]
pub enum Expr {
    /// A number literal.
    Lit(f32),
    /// The `x` coordinate.
    X,
    /// The `y` coordinate.
    Y,
    /// `a + b`.
    Add(Box<Expr>, Box<Expr>),
    /// `a - b`.
    Sub(Box<Expr>, Box<Expr>),
    /// `a * b`.
    Mul(Box<Expr>, Box<Expr>),
    /// `a / b` (0 when `|b|` is below 1e-10).
    Div(Box<Expr>, Box<Expr>),
    /// `a ^ b`.
    Pow(Box<Expr>, Box<Expr>),
    /// `-a`.
    Neg(Box<Expr>),
    /// `sin(a)`, radians.
    Sin(Box<Expr>),
    /// `cos(a)`, radians.
    Cos(Box<Expr>),
    /// `tan(a)`, radians.
    Tan(Box<Expr>),
    /// `sqrt(|a|)`.
    Sqrt(Box<Expr>),
    /// `|a|`.
    Abs(Box<Expr>),
    /// `e^a`.
    Exp(Box<Expr>),
    /// Natural log of `|a|` (`ln` and `log` both parse to this).
    Ln(Box<Expr>),
}

impl Expr {
    /// Evaluate the expression at `(x, y)`.
    pub fn eval(&self, x: f32, y: f32) -> f32 {
        match self {
            Expr::Lit(v) => *v,
            Expr::X => x,
            Expr::Y => y,
            Expr::Add(a, b) => a.eval(x, y) + b.eval(x, y),
            Expr::Sub(a, b) => a.eval(x, y) - b.eval(x, y),
            Expr::Mul(a, b) => a.eval(x, y) * b.eval(x, y),
            Expr::Div(a, b) => {
                let denom = b.eval(x, y);
                if denom.abs() < 1e-10 {
                    0.0
                } else {
                    a.eval(x, y) / denom
                }
            }
            Expr::Pow(base, exp) => base.eval(x, y).powf(exp.eval(x, y)),
            Expr::Neg(e) => -e.eval(x, y),
            Expr::Sin(e) => e.eval(x, y).sin(),
            Expr::Cos(e) => e.eval(x, y).cos(),
            Expr::Tan(e) => e.eval(x, y).tan(),
            Expr::Sqrt(e) => e.eval(x, y).abs().sqrt(),
            Expr::Abs(e) => e.eval(x, y).abs(),
            Expr::Exp(e) => e.eval(x, y).exp(),
            Expr::Ln(e) => e.eval(x, y).abs().ln(),
        }
    }
}

// ── Expression parser ─────────────────────────────────────────────────────────

/// Parse a mathematical expression string into an [`Expr`] AST.
///
/// Grammar (recursive descent):
/// ```text
/// expr   = term { ('+' | '-') term }
/// term   = unary { ('*' | '/') unary }
/// unary  = '-' unary | factor
/// factor = base ('^' unary)?
/// base   = number | 'x' | 'y' | 'pi' | 'e' | func '(' expr ')' | '(' expr ')'
/// ```
///
/// Numbers may use scientific notation (`1e-3`). Nesting deeper than 64
/// levels is rejected.
pub fn parse_expression(input: &str) -> Result<Expr, String> {
    let tokens = tokenize(input)?;
    let mut p = Parser {
        tokens: &tokens,
        pos: 0,
        depth: 0,
    };
    let expr = p.expr()?;
    if p.pos < tokens.len() {
        return Err(format!(
            "unexpected token at position {}: {:?}",
            p.pos, tokens[p.pos]
        ));
    }
    Ok(expr)
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f32),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            c if c.is_whitespace() => {
                i += 1;
            }
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '^' => {
                tokens.push(Token::Caret);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                // Optional exponent: e or E, an optional sign, then digits.
                if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                    let mut j = i + 1;
                    if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                        j += 1;
                    }
                    if j < chars.len() && chars[j].is_ascii_digit() {
                        while j < chars.len() && chars[j].is_ascii_digit() {
                            j += 1;
                        }
                        i = j;
                    }
                }
                let s: String = chars[start..i].iter().collect();
                let v: f32 = s.parse().map_err(|_| format!("invalid number: {s}"))?;
                tokens.push(Token::Num(v));
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::Ident(s));
            }
            c => return Err(format!("unexpected character: '{c}'")),
        }
    }
    Ok(tokens)
}

/// Deepest nesting of parentheses, function calls, unary minus and powers
/// accepted, so hostile input cannot overflow the stack.
const MAX_DEPTH: usize = 64;

struct Parser<'t> {
    tokens: &'t [Token],
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn nested<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T, String>) -> Result<T, String> {
        if self.depth >= MAX_DEPTH {
            return Err(format!("expression nested deeper than {MAX_DEPTH} levels"));
        }
        self.depth += 1;
        let r = f(self);
        self.depth -= 1;
        r
    }

    /// `expr = term { ('+' | '-') term }`
    fn expr(&mut self) -> Result<Expr, String> {
        let mut left = self.term()?;
        loop {
            match self.peek() {
                Some(Token::Plus) => {
                    self.pos += 1;
                    left = Expr::Add(Box::new(left), Box::new(self.term()?));
                }
                Some(Token::Minus) => {
                    self.pos += 1;
                    left = Expr::Sub(Box::new(left), Box::new(self.term()?));
                }
                _ => return Ok(left),
            }
        }
    }

    /// `term = unary { ('*' | '/') unary }`
    fn term(&mut self) -> Result<Expr, String> {
        let mut left = self.unary()?;
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.pos += 1;
                    left = Expr::Mul(Box::new(left), Box::new(self.unary()?));
                }
                Some(Token::Slash) => {
                    self.pos += 1;
                    left = Expr::Div(Box::new(left), Box::new(self.unary()?));
                }
                _ => return Ok(left),
            }
        }
    }

    /// `unary = '-' unary | factor`: minus binds looser than `^`.
    fn unary(&mut self) -> Result<Expr, String> {
        if self.peek() == Some(&Token::Minus) {
            self.pos += 1;
            let inner = self.nested(|p| p.unary())?;
            return Ok(Expr::Neg(Box::new(inner)));
        }
        self.factor()
    }

    /// `factor = base ('^' unary)?`, right-associative.
    fn factor(&mut self) -> Result<Expr, String> {
        let base = self.base()?;
        if self.peek() == Some(&Token::Caret) {
            self.pos += 1;
            let exp = self.nested(|p| p.unary())?;
            return Ok(Expr::Pow(Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    /// `base = number | 'x' | 'y' | 'pi' | 'e' | func '(' expr ')' | '(' expr ')'`
    fn base(&mut self) -> Result<Expr, String> {
        let Some(tok) = self.peek().cloned() else {
            return Err("unexpected end of expression".into());
        };
        self.pos += 1;
        match tok {
            Token::Num(v) => Ok(Expr::Lit(v)),
            Token::Ident(name) => match name.as_str() {
                "x" => Ok(Expr::X),
                "y" => Ok(Expr::Y),
                "pi" => Ok(Expr::Lit(std::f32::consts::PI)),
                "e" => Ok(Expr::Lit(std::f32::consts::E)),
                func => {
                    let make: fn(Box<Expr>) -> Expr = match func {
                        "sin" => Expr::Sin,
                        "cos" => Expr::Cos,
                        "tan" => Expr::Tan,
                        "sqrt" => Expr::Sqrt,
                        "abs" => Expr::Abs,
                        "exp" => Expr::Exp,
                        "ln" | "log" => Expr::Ln,
                        other => return Err(format!("unknown function: '{other}'")),
                    };
                    if self.peek() != Some(&Token::LParen) {
                        return Err(format!("expected '(' after function '{func}'"));
                    }
                    self.pos += 1;
                    let arg = self.nested(|p| p.expr())?;
                    if self.peek() != Some(&Token::RParen) {
                        return Err(format!("expected ')' after argument of '{func}'"));
                    }
                    self.pos += 1;
                    Ok(make(Box::new(arg)))
                }
            },
            Token::LParen => {
                let inner = self.nested(|p| p.expr())?;
                if self.peek() != Some(&Token::RParen) {
                    return Err("expected ')'".into());
                }
                self.pos += 1;
                Ok(inner)
            }
            other => Err(format!("unexpected token: {other:?}")),
        }
    }
}

// ── Perlin noise ──────────────────────────────────────────────────────────────

/// Minimal value-noise implementation (no external dep required).
fn value_noise_2d(x: f32, y: f32, seed: u32) -> f32 {
    fn hash(xi: i32, yi: i32, seed: u32) -> f32 {
        let mut h = seed
            .wrapping_add(xi.unsigned_abs())
            .wrapping_mul(2246822519);
        h ^= yi.unsigned_abs().wrapping_mul(3266489917);
        h = h.wrapping_mul(668265263);
        h ^= h >> 15;
        h = h.wrapping_mul(2246822519);
        h ^= h >> 13;
        (h >> 8) as f32 / 16777215.0 * 2.0 - 1.0
    }
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    // Smooth step.
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    let n00 = hash(xi, yi, seed);
    let n10 = hash(xi + 1, yi, seed);
    let n01 = hash(xi, yi + 1, seed);
    let n11 = hash(xi + 1, yi + 1, seed);
    let bottom = n00 * (1.0 - ux) + n10 * ux;
    let top = n01 * (1.0 - ux) + n11 * ux;
    bottom * (1.0 - uy) + top * uy
}

/// Fractal (fBm) noise: sum of `octaves` noise layers at increasing frequency.
pub fn fbm(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let mut value = 0.0_f32;
    let mut amplitude = 1.0_f32;
    let mut frequency = 1.0_f32;
    let mut max_val = 0.0_f32;
    for oct in 0..octaves {
        value += value_noise_2d(x * frequency, y * frequency, seed.wrapping_add(oct)) * amplitude;
        max_val += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    if max_val > 0.0 {
        value / max_val
    } else {
        0.0
    }
}

// ── Procedural surface ────────────────────────────────────────────────────────

/// Method used to compute `z = f(x, y)`.
#[derive(Clone)]
pub enum ProceduralMode {
    /// Fractal Brownian motion noise.
    Perlin {
        /// Spatial frequency: larger values give smaller hills.
        scale: f32,
        /// Number of noise layers summed (each at double frequency).
        octaves: u32,
        /// Height multiplier applied to the noise.
        amplitude: f32,
        /// Seed that selects the noise pattern.
        seed: u32,
    },
    /// Runtime-parsed mathematical expression.
    Expression(Expr),
}

/// A height-field surface `z = f(x, y)` on a flat `[u_min, u_max] × [v_min, v_max]` domain.
///
/// Implements the full [`Surface`] trait so it can be used with the geodesic
/// integrator and the wgpu renderer.
#[derive(Clone)]
pub struct ProceduralSurface {
    /// How the height is computed.
    pub mode: ProceduralMode,
    /// Smallest `x` (the `u` parameter) of the domain.
    pub u_min: f32,
    /// Largest `x` of the domain.
    pub u_max: f32,
    /// Smallest `y` (the `v` parameter) of the domain.
    pub v_min: f32,
    /// Largest `y` of the domain.
    pub v_max: f32,
    /// Finite difference step for metric / Christoffel computation.
    eps: f32,
}

impl ProceduralSurface {
    /// Create a Perlin / fBm noise surface.
    pub fn perlin(scale: f32, octaves: u32, amplitude: f32) -> Self {
        Self {
            mode: ProceduralMode::Perlin {
                scale,
                octaves,
                amplitude,
                seed: 42,
            },
            u_min: -5.0,
            u_max: 5.0,
            v_min: -5.0,
            v_max: 5.0,
            eps: 1e-3,
        }
    }

    /// Create a surface from a user expression string `"z = f(x, y)"`.
    pub fn from_expression(expr_str: &str) -> Result<Self, String> {
        let expr = parse_expression(expr_str)?;
        Ok(Self {
            mode: ProceduralMode::Expression(expr),
            u_min: -5.0,
            u_max: 5.0,
            v_min: -5.0,
            v_max: 5.0,
            eps: 1e-3,
        })
    }

    /// Evaluate the height function at `(u, v)`.
    pub fn height(&self, u: f32, v: f32) -> f32 {
        // Map u/v to x/y world coords.
        let x = u;
        let y = v;
        match &self.mode {
            ProceduralMode::Perlin {
                scale,
                octaves,
                amplitude,
                seed,
            } => fbm(x * scale, y * scale, *octaves, *seed) * amplitude,
            ProceduralMode::Expression(expr) => {
                let z = expr.eval(x, y);
                if z.is_finite() {
                    z
                } else {
                    0.0
                }
            }
        }
    }

    /// Numeric partial derivatives ∂z/∂u and ∂z/∂v.
    fn dz(&self, u: f32, v: f32) -> (f32, f32) {
        let dzu = (self.height(u + self.eps, v) - self.height(u - self.eps, v)) / (2.0 * self.eps);
        let dzv = (self.height(u, v + self.eps) - self.height(u, v - self.eps)) / (2.0 * self.eps);
        (dzu, dzv)
    }
}

impl Surface for ProceduralSurface {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(u, v, self.height(u, v))
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let (dzu, dzv) = self.dz(u, v);
        // g_uu = 1 + (∂z/∂u)², g_vv = 1 + (∂z/∂v)², g_uv = (∂z/∂u)(∂z/∂v)
        [[1.0 + dzu * dzu, dzu * dzv], [dzu * dzv, 1.0 + dzv * dzv]]
    }

    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        // Numeric Christoffel symbols via finite differences of the metric.
        let eps = self.eps;
        let guu = |u: f32, v: f32| self.metric(u, v)[0][0];
        let guv = |u: f32, v: f32| self.metric(u, v)[0][1];
        let gvv = |u: f32, v: f32| self.metric(u, v)[1][1];

        let dg_uu_u = (guu(u + eps, v) - guu(u - eps, v)) / (2.0 * eps);
        let dg_uu_v = (guu(u, v + eps) - guu(u, v - eps)) / (2.0 * eps);
        let dg_uv_u = (guv(u + eps, v) - guv(u - eps, v)) / (2.0 * eps);
        let dg_uv_v = (guv(u, v + eps) - guv(u, v - eps)) / (2.0 * eps);
        let dg_vv_u = (gvv(u + eps, v) - gvv(u - eps, v)) / (2.0 * eps);
        let dg_vv_v = (gvv(u, v + eps) - gvv(u, v - eps)) / (2.0 * eps);

        // Metric inverse (2×2).
        let g = self.metric(u, v);
        let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
        let inv_det = if det.abs() < 1e-10 { 0.0 } else { 1.0 / det };
        let g_inv = [
            [g[1][1] * inv_det, -g[0][1] * inv_det],
            [-g[1][0] * inv_det, g[0][0] * inv_det],
        ];

        // Γ^k_ij = ½ g^{kl} (∂_i g_{lj} + ∂_j g_{li} − ∂_l g_{ij})
        let dg = [
            [[dg_uu_u, dg_uv_u], [dg_uv_u, dg_vv_u]], // ∂_u g_{lj}
            [[dg_uu_v, dg_uv_v], [dg_uv_v, dg_vv_v]], // ∂_v g_{lj}
        ];

        let mut gamma = [[[0.0f32; 2]; 2]; 2];
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    let mut sum = 0.0;
                    for l in 0..2 {
                        sum += g_inv[k][l] * (dg[i][l][j] + dg[j][l][i] - dg[l][i][j]);
                    }
                    gamma[k][i][j] = 0.5 * sum;
                }
            }
        }
        gamma
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        (
            u.clamp(self.u_min, self.u_max),
            v.clamp(self.v_min, self.v_max),
        )
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        let (dzu, dzv) = self.dz(u, v);
        Vec3::new(-dzu, -dzv, 1.0).normalize()
    }

    fn random_position(&self, rng: &mut dyn RngCore) -> (f32, f32) {
        use rand::Rng;
        let u = rng.gen::<f32>() * (self.u_max - self.u_min) + self.u_min;
        let v = rng.gen::<f32>() * (self.v_max - self.v_min) + self.v_min;
        (u, v)
    }

    fn random_tangent(&self, u: f32, v: f32, rng: &mut dyn RngCore) -> (f32, f32) {
        use rand::Rng;
        let theta = rng.gen::<f32>() * std::f32::consts::TAU;
        let g = self.metric(u, v);
        // Unit tangent satisfying g_ij t^i t^j = 1.
        let du = theta.cos();
        let dv = theta.sin();
        let spd = (g[0][0] * du * du + 2.0 * g[0][1] * du * dv + g[1][1] * dv * dv)
            .max(1e-10)
            .sqrt();
        (du / spd, dv / spd)
    }

    fn mesh_vertices(&self, u_steps: u32, v_steps: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut indices = Vec::new();
        let u_steps = u_steps.max(2);
        let v_steps = v_steps.max(2);
        for vi in 0..=v_steps {
            for ui in 0..=u_steps {
                let u = self.u_min + (ui as f32 / u_steps as f32) * (self.u_max - self.u_min);
                let v = self.v_min + (vi as f32 / v_steps as f32) * (self.v_max - self.v_min);
                let p = self.position(u, v);
                verts.push([p.x, p.y, p.z]);
            }
        }
        let row = u_steps + 1;
        for vi in 0..v_steps {
            for ui in 0..u_steps {
                let i00 = vi * row + ui;
                let i10 = i00 + 1;
                let i01 = i00 + row;
                let i11 = i01 + 1;
                indices.extend_from_slice(&[i00, i10, i01, i10, i11, i01]);
            }
        }
        (verts, indices)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(src: &str, x: f32, y: f32) -> f32 {
        parse_expression(src).expect("parse").eval(x, y)
    }

    // Unary minus binds looser than `^`: -x^2 is -(x^2), as in maths and
    // most languages. It used to parse as (-x)^2.
    #[test]
    fn bug_unary_minus_vs_power() {
        assert_eq!(ev("-x^2", 3.0, 0.0), -9.0);
        assert_eq!(ev("2^-1", 0.0, 0.0), 0.5);
        assert_eq!(ev("-(x)", 3.0, 0.0), -3.0);
    }

    // Scientific notation used to be split into a number and the constant e.
    #[test]
    fn bug_scientific_notation() {
        assert!((ev("1e-3", 0.0, 0.0) - 0.001).abs() < 1e-9);
        assert!((ev("2.5E2 * x", 2.0, 0.0) - 500.0).abs() < 1e-3);
        // The constant e still works on its own.
        assert!((ev("e", 0.0, 0.0) - std::f32::consts::E).abs() < 1e-6);
    }

    #[test]
    fn bug_deep_nesting_is_error() {
        let deep = format!("{}x{}", "(".repeat(100_000), ")".repeat(100_000));
        assert!(parse_expression(&deep).is_err());
        let minus = format!("{}x", "-".repeat(100_000));
        assert!(parse_expression(&minus).is_err());
        assert_eq!(ev("((x))", 4.0, 0.0), 4.0);
    }

    #[test]
    fn test_expression_parse_and_eval() {
        let expr = parse_expression("sin(x) * cos(y)").unwrap();
        let val = expr.eval(0.0, 0.0);
        assert!((val - 0.0).abs() < 1e-5, "sin(0)*cos(0) should be 0: {val}");

        let expr2 = parse_expression("x^2 + y^2").unwrap();
        let val2 = expr2.eval(3.0, 4.0);
        assert!((val2 - 25.0).abs() < 1e-4, "3^2+4^2 should be 25: {val2}");
    }

    #[test]
    fn test_expression_parse_pi() {
        let expr = parse_expression("sin(pi)").unwrap();
        let val = expr.eval(0.0, 0.0);
        assert!(val.abs() < 1e-5, "sin(pi) should be ~0: {val}");
    }

    #[test]
    fn test_expression_parse_error() {
        assert!(parse_expression("sin(x +").is_err());
        assert!(parse_expression("unknown(x)").is_err());
    }

    #[test]
    fn test_perlin_surface_position_finite() {
        let surf = ProceduralSurface::perlin(1.0, 4, 1.0);
        let p = surf.position(1.0, 2.0);
        assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
    }

    #[test]
    fn test_expression_surface_height() {
        let surf = ProceduralSurface::from_expression("x^2 - y^2").unwrap();
        let h = surf.height(3.0, 2.0);
        assert!((h - 5.0).abs() < 1e-3, "height at (3,2) should be 5: {h}");
    }

    #[test]
    fn test_metric_positive_definite() {
        let surf = ProceduralSurface::perlin(0.5, 2, 0.5);
        let g = surf.metric(1.0, 1.0);
        let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
        assert!(det > 0.0, "metric should be positive definite");
    }

    #[test]
    fn test_normal_unit_length() {
        let surf = ProceduralSurface::from_expression("0.5 * sin(x) * cos(y)").unwrap();
        let n = surf.normal(1.0, 1.0);
        let len = n.length();
        assert!(
            (len - 1.0).abs() < 1e-4,
            "normal should be unit length: {len}"
        );
    }

    #[test]
    fn test_mesh_vertices_count() {
        let surf = ProceduralSurface::perlin(1.0, 2, 1.0);
        let (verts, indices) = surf.mesh_vertices(4, 4);
        assert_eq!(verts.len(), 5 * 5, "should have (4+1)^2 vertices");
        assert_eq!(indices.len(), 4 * 4 * 6, "should have 4*4*6 indices");
    }
}
