//! Lua-scriptable custom surface (`lua` feature flag required).
//!
//! Users write a Lua script that exports two functions:
//!
//! ```lua
//! function metric(u, v)
//!   return {g_uu=1.0, g_uv=0.0, g_vu=0.0, g_vv=math.sin(u)^2}
//! end
//!
//! -- optional: return Christoffel symbols directly to skip numeric derivation
//! function christoffel(u, v)
//!   return {
//!     g000=0.0, g001=0.0, g010=0.0, g011=0.0,
//!     g100=0.0, g101=0.0, g110=0.0, g111=0.0,
//!   }
//! end
//! ```
//!
//! If `christoffel` is absent the symbols are derived numerically from the
//! metric using finite differences.  If Lua evaluation fails at any point the
//! surface falls back to the default torus metric.

#[cfg(feature = "lua")]
mod inner {
    use crate::surface::Surface;
    use glam::Vec3;
    use mlua::{Lua, Result as LuaResult, Table, Value};
    use std::f32::consts::TAU;
    use std::sync::{Arc, Mutex};

    /// Error type returned when loading a Lua surface script.
    #[derive(Debug)]
    pub enum LuaError {
        /// The mlua runtime reported an error.
        MluaError(mlua::Error),
        /// The Lua table returned by `metric()` was missing an expected key.
        MissingKey(&'static str),
    }

    impl std::fmt::Display for LuaError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                LuaError::MluaError(e) => write!(f, "Lua error: {e}"),
                LuaError::MissingKey(k) => write!(f, "missing key in Lua table: {k}"),
            }
        }
    }

    impl std::error::Error for LuaError {}

    impl From<mlua::Error> for LuaError {
        fn from(e: mlua::Error) -> Self {
            LuaError::MluaError(e)
        }
    }

    /// Lua-defined metric components returned by `metric(u, v)`.
    #[derive(Clone, Copy, Debug)]
    struct MetricResult {
        g_uu: f32,
        g_uv: f32,
        g_vu: f32,
        g_vv: f32,
    }

    impl MetricResult {
        /// Decode a Lua table `{g_uu, g_uv, g_vu, g_vv}`.
        fn from_table(t: &Table) -> Result<Self, LuaError> {
            let get = |key: &'static str| -> Result<f32, LuaError> {
                match t.get::<Value>(key)? {
                    Value::Number(n) => Ok(n as f32),
                    Value::Integer(n) => Ok(n as f32),
                    _ => Err(LuaError::MissingKey(key)),
                }
            };
            Ok(Self {
                g_uu: get("g_uu")?,
                g_uv: get("g_uv")?,
                g_vu: get("g_vu")?,
                g_vv: get("g_vv")?,
            })
        }
    }

    /// Internal shared state held behind a mutex so `LuaSurface` is `Send + Sync`.
    struct LuaState {
        lua: Lua,
        has_christoffel: bool,
    }

    impl LuaState {
        fn new(script: &str) -> LuaResult<Self> {
            let lua = Lua::new();
            lua.load(script).exec()?;
            let has_christoffel = lua
                .globals()
                .get::<Value>("christoffel")
                .map(|v| matches!(v, Value::Function(_)))
                .unwrap_or(false);
            Ok(Self { lua, has_christoffel })
        }

        /// Call the Lua `metric` function and parse the result.
        fn metric(&self, u: f32, v: f32) -> Result<MetricResult, LuaError> {
            let func: mlua::Function = self.lua.globals().get("metric")?;
            let table: Table = func.call((u as f64, v as f64))?;
            MetricResult::from_table(&table)
        }

        /// Call the optional Lua `christoffel` function.
        fn christoffel(&self, u: f32, v: f32) -> Option<[[[f32; 2]; 2]; 2]> {
            if !self.has_christoffel {
                return None;
            }
            let func: mlua::Function = self.lua.globals().get("christoffel").ok()?;
            let table: Table = func.call((u as f64, v as f64)).ok()?;
            let g = |key: &'static str| -> f32 {
                match table.get::<Value>(key).ok()? {
                    Value::Number(n) => Some(n as f32),
                    Value::Integer(n) => Some(n as f32),
                    _ => None,
                }
                .unwrap_or(0.0)
            };
            Some([
                [[g("g000"), g("g001")], [g("g010"), g("g011")]],
                [[g("g100"), g("g101")], [g("g110"), g("g111")]],
            ])
        }
    }

    /// A surface whose metric is defined by a Lua script at runtime.
    ///
    /// Falls back to the torus metric `g = diag((2+cos v)^2, 1)` if the Lua
    /// script produces an error or a degenerate metric.
    pub struct LuaSurface {
        state: Arc<Mutex<LuaState>>,
        /// The script source, kept for hot-reload.
        script: String,
    }

    impl LuaSurface {
        /// Load a Lua surface from `script` source text.
        ///
        /// Returns `Err` only if the Lua sandbox itself fails to initialise or
        /// the script has a top-level syntax / runtime error.
        pub fn from_script(script: &str) -> Result<Self, LuaError> {
            let state = LuaState::new(script)?;
            Ok(Self {
                state: Arc::new(Mutex::new(state)),
                script: script.to_owned(),
            })
        }

        /// Hot-reload: replace the Lua script source.
        ///
        /// If loading fails the existing script continues to run unchanged and
        /// a warning is emitted via `tracing`.
        pub fn reload(&mut self, new_script: &str) {
            match LuaState::new(new_script) {
                Ok(new_state) => {
                    self.script = new_script.to_owned();
                    if let Ok(mut guard) = self.state.lock() {
                        *guard = new_state;
                    }
                }
                Err(e) => {
                    tracing::warn!("LuaSurface hot-reload failed, keeping old script: {e}");
                }
            }
        }

        /// Returns the current Lua source text.
        pub fn script(&self) -> &str {
            &self.script
        }

        /// Evaluate the metric at `(u, v)`, falling back to a torus metric on error.
        fn eval_metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
            let guard = match self.state.lock() {
                Ok(g) => g,
                Err(_) => return Self::fallback_metric(u, v),
            };
            match guard.metric(u, v) {
                Ok(m) => {
                    // Guard against degenerate or NaN metric.
                    let det = m.g_uu * m.g_vv - m.g_uv * m.g_vu;
                    if det > 1e-10 && m.g_uu.is_finite() && m.g_vv.is_finite() {
                        [[m.g_uu, m.g_uv], [m.g_vu, m.g_vv]]
                    } else {
                        tracing::warn!("LuaSurface: degenerate metric at ({u},{v}), using fallback");
                        Self::fallback_metric(u, v)
                    }
                }
                Err(e) => {
                    tracing::warn!("LuaSurface metric error at ({u},{v}): {e}, using fallback");
                    Self::fallback_metric(u, v)
                }
            }
        }

        /// Torus metric used as a safe fallback: `g = diag((2+0.7 cos v)^2, 0.49)`.
        fn fallback_metric(u: f32, v: f32) -> [[f32; 2]; 2] {
            let _ = u;
            let f = 2.0 + 0.7 * v.cos();
            [[f * f, 0.0], [0.0, 0.49_f32]]
        }

        /// Numerically differentiate the metric to obtain Christoffel symbols.
        ///
        /// Uses a central finite difference with step `h = 1e-4`.
        fn numeric_christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
            const H: f32 = 1e-4;
            let g    = self.eval_metric(u,     v    );
            let g_u1 = self.eval_metric(u + H, v    );
            let g_u0 = self.eval_metric(u - H, v    );
            let g_v1 = self.eval_metric(u,     v + H);
            let g_v0 = self.eval_metric(u,     v - H);

            // ∂g_{ij}/∂u and ∂g_{ij}/∂v by central differences.
            let dg_du = |i: usize, j: usize| (g_u1[i][j] - g_u0[i][j]) / (2.0 * H);
            let dg_dv = |i: usize, j: usize| (g_v1[i][j] - g_v0[i][j]) / (2.0 * H);

            let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
            // Inverse of 2×2 symmetric metric.
            let inv = if det.abs() > 1e-12 {
                [
                    [ g[1][1] / det, -g[0][1] / det],
                    [-g[1][0] / det,  g[0][0] / det],
                ]
            } else {
                [[1.0, 0.0], [0.0, 1.0]]
            };

            // Γ^k_ij = ½ g^{kl} (∂_i g_{lj} + ∂_j g_{li} − ∂_l g_{ij})
            let partial = |coord: usize| -> [[f32; 2]; 2] {
                let dg = if coord == 0 { dg_du } else { dg_dv };
                [
                    [dg(0, 0), dg(0, 1)],
                    [dg(1, 0), dg(1, 1)],
                ]
            };
            let dg_by = [partial(0), partial(1)]; // dg_by[l][i][j] = ∂_l g_{ij}

            let mut gamma = [[[0.0f32; 2]; 2]; 2];
            for k in 0..2 {
                for i in 0..2 {
                    for j in 0..2 {
                        let mut sum = 0.0f32;
                        for l in 0..2 {
                            // ∂_i g_{lj} + ∂_j g_{li} − ∂_l g_{ij}
                            let term = dg_by[i][l][j] + dg_by[j][l][i] - dg_by[l][i][j];
                            sum += inv[k][l] * term;
                        }
                        gamma[k][i][j] = 0.5 * sum;
                    }
                }
            }
            gamma
        }
    }

    impl Surface for LuaSurface {
        /// Position is not analytically defined by the Lua script; we return the
        /// identity embedding `(u, v) → (u, v, 0)` scaled to look reasonable.
        fn position(&self, u: f32, v: f32) -> Vec3 {
            Vec3::new(u, v, 0.0)
        }

        fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
            self.eval_metric(u, v)
        }

        fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
            let guard = match self.state.lock() {
                Ok(g) => g,
                Err(_) => return self.numeric_christoffel(u, v),
            };
            if let Some(c) = guard.christoffel(u, v) {
                return c;
            }
            drop(guard);
            self.numeric_christoffel(u, v)
        }

        fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
            (u.rem_euclid(TAU), v.rem_euclid(TAU))
        }

        fn normal(&self, _u: f32, _v: f32) -> Vec3 {
            Vec3::Z
        }

        fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32) {
            use rand::Rng;
            (rng.gen_range(0.0..TAU), rng.gen_range(0.0..TAU))
        }

        fn random_tangent(&self, u: f32, v: f32, rng: &mut dyn rand::RngCore) -> (f32, f32) {
            use rand::Rng;
            let angle: f32 = rng.gen_range(0.0..TAU);
            let g = self.eval_metric(u, v);
            let g00 = g[0][0].max(1e-6);
            let g11 = g[1][1].max(1e-6);
            (angle.cos() / g00.sqrt(), angle.sin() / g11.sqrt())
        }

        fn mesh_vertices(&self, u_steps: u32, v_steps: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
            let mut verts = Vec::new();
            let mut indices = Vec::new();
            for i in 0..=u_steps {
                for j in 0..=v_steps {
                    let u = (i as f32 / u_steps as f32) * TAU;
                    let v = (j as f32 / v_steps as f32) * TAU;
                    let p = self.position(u, v);
                    verts.push([p.x, p.y, p.z]);
                }
            }
            for i in 0..u_steps {
                for j in 0..v_steps {
                    let a = i * (v_steps + 1) + j;
                    let b = a + 1;
                    let c = (i + 1) * (v_steps + 1) + j;
                    let d = c + 1;
                    indices.extend_from_slice(&[a, b, c, b, d, c]);
                }
            }
            (verts, indices)
        }
    }
}

#[cfg(feature = "lua")]
pub use inner::{LuaError, LuaSurface};

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "lua"))]
mod tests {
    use super::*;
    use crate::surface::Surface;

    const FLAT_SCRIPT: &str = r#"
        function metric(u, v)
          return {g_uu=1.0, g_uv=0.0, g_vu=0.0, g_vv=1.0}
        end
    "#;

    const CHRISTOFFEL_SCRIPT: &str = r#"
        function metric(u, v)
          return {g_uu=1.0, g_uv=0.0, g_vu=0.0, g_vv=1.0}
        end
        function christoffel(u, v)
          return {
            g000=0.0, g001=0.0, g010=0.0, g011=0.0,
            g100=0.0, g101=0.0, g110=0.0, g111=0.0,
          }
        end
    "#;

    const BAD_SCRIPT: &str = r#"
        function metric(u, v)
          return {g_uu=0.0, g_uv=0.0, g_vu=0.0, g_vv=0.0}  -- degenerate
        end
    "#;

    #[test]
    fn from_script_ok() {
        let s = LuaSurface::from_script(FLAT_SCRIPT);
        assert!(s.is_ok(), "should load flat metric script");
    }

    #[test]
    fn flat_metric_identity() {
        let s = LuaSurface::from_script(FLAT_SCRIPT).unwrap();
        let g = s.metric(1.0, 2.0);
        assert!((g[0][0] - 1.0).abs() < 1e-5);
        assert!((g[1][1] - 1.0).abs() < 1e-5);
        assert!(g[0][1].abs() < 1e-5);
    }

    #[test]
    fn degenerate_metric_falls_back() {
        let s = LuaSurface::from_script(BAD_SCRIPT).unwrap();
        // Degenerate Lua metric => fallback torus metric => g[0][0] > 0.
        let g = s.metric(0.0, 0.0);
        assert!(g[0][0] > 0.0, "fallback metric should have positive g_00");
    }

    #[test]
    fn numeric_christoffel_flat_is_zero() {
        let s = LuaSurface::from_script(FLAT_SCRIPT).unwrap();
        let c = s.christoffel(1.0, 1.0);
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    assert!(
                        c[k][i][j].abs() < 1e-3,
                        "Γ^{k}_{i}{j} = {} on flat metric",
                        c[k][i][j]
                    );
                }
            }
        }
    }

    #[test]
    fn lua_christoffel_used_when_present() {
        let s = LuaSurface::from_script(CHRISTOFFEL_SCRIPT).unwrap();
        let c = s.christoffel(0.5, 0.5);
        // All returned as 0.0 by the Lua function.
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    assert!(c[k][i][j].abs() < 1e-6);
                }
            }
        }
    }

    #[test]
    fn reload_replaces_metric() {
        let mut s = LuaSurface::from_script(FLAT_SCRIPT).unwrap();
        let new_script = r#"
            function metric(u, v)
              return {g_uu=4.0, g_uv=0.0, g_vu=0.0, g_vv=4.0}
            end
        "#;
        s.reload(new_script);
        let g = s.metric(0.0, 0.0);
        assert!((g[0][0] - 4.0).abs() < 1e-4, "g_uu should be 4.0 after reload");
    }

    #[test]
    fn reload_bad_script_keeps_old() {
        let mut s = LuaSurface::from_script(FLAT_SCRIPT).unwrap();
        s.reload("this is not valid lua @@@@");
        // Old flat metric still works.
        let g = s.metric(0.0, 0.0);
        assert!((g[0][0] - 1.0).abs() < 1e-4, "old metric should still work");
    }

    #[test]
    fn mesh_vertices_count() {
        let s = LuaSurface::from_script(FLAT_SCRIPT).unwrap();
        let (verts, indices) = s.mesh_vertices(8, 8);
        assert_eq!(verts.len(), 9 * 9);
        assert_eq!(indices.len(), 8 * 8 * 6);
    }
}
