//! Pattern Composer — composites multiple weighted pattern layers into one image.

use crate::gradient::Gradient;
use std::sync::Arc;

// ── BlendMode ─────────────────────────────────────────────────────────────────

/// Blending mode used to combine a new layer on top of a base value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// `(base + layer).min(1.0)`
    Add,
    /// `base * layer`
    Multiply,
    /// `1 - (1-base)*(1-layer)`
    Screen,
    /// if base < 0.5: `2*base*layer`, else `1 - 2*(1-base)*(1-layer)`
    Overlay,
    /// `(base - layer).abs()`
    Difference,
}

impl BlendMode {
    /// Apply this blend mode to a base and layer value in [0, 1].
    pub fn apply(self, base: f32, layer: f32) -> f32 {
        match self {
            BlendMode::Add => (base + layer).min(1.0),
            BlendMode::Multiply => base * layer,
            BlendMode::Screen => 1.0 - (1.0 - base) * (1.0 - layer),
            BlendMode::Overlay => {
                if base < 0.5 {
                    2.0 * base * layer
                } else {
                    1.0 - 2.0 * (1.0 - base) * (1.0 - layer)
                }
            }
            BlendMode::Difference => (base - layer).abs(),
        }
    }
}

// ── Layer ─────────────────────────────────────────────────────────────────────

/// A single compositing layer: a pattern function, a weight, and a blend mode.
pub struct Layer {
    /// Pattern function: `(x_norm, y_norm) -> value in [0, 1]` where x and y
    /// are normalised to [0, 1] over the rendered image.
    pub pattern_fn: Arc<dyn Fn(f32, f32) -> f32 + Send + Sync>,
    /// Weight applied to this layer's output before blending.
    pub weight: f32,
    /// How this layer is composited on top of the accumulated result.
    pub blend_mode: BlendMode,
}

impl Layer {
    pub fn new(
        pattern_fn: impl Fn(f32, f32) -> f32 + Send + Sync + 'static,
        weight: f32,
        blend_mode: BlendMode,
    ) -> Self {
        Self {
            pattern_fn: Arc::new(pattern_fn),
            weight: weight.clamp(0.0, 1.0),
            blend_mode,
        }
    }
}

// ── PatternComposer ───────────────────────────────────────────────────────────

/// Composites multiple pattern layers into a single-channel float image.
pub struct PatternComposer {
    layers: Vec<Layer>,
}

impl PatternComposer {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Add a layer to the compositor.
    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    /// Render the composite to a flat `Vec<f32>` of values in [0, 1],
    /// row-major, `width * height` elements.
    pub fn render(&self, width: u32, height: u32) -> Vec<f32> {
        let w = width as usize;
        let h = height as usize;
        let mut pixels = vec![0.0_f32; w * h];

        for (idx, px) in pixels.iter_mut().enumerate() {
            let row = idx / w;
            let col = idx % w;
            let xn = if w > 1 {
                col as f32 / (w - 1) as f32
            } else {
                0.5
            };
            let yn = if h > 1 {
                row as f32 / (h - 1) as f32
            } else {
                0.5
            };

            let mut acc = 0.0_f32;
            for layer in &self.layers {
                let raw = (layer.pattern_fn)(xn, yn).clamp(0.0, 1.0) * layer.weight;
                acc = layer.blend_mode.apply(acc, raw);
            }
            *px = acc.clamp(0.0, 1.0);
        }
        pixels
    }

    /// Render the composite and map through a gradient to produce RGB pixels.
    pub fn render_rgb(&self, width: u32, height: u32, gradient: &Gradient) -> Vec<[u8; 3]> {
        self.render(width, height)
            .into_iter()
            .map(|v| gradient.sample(v))
            .collect()
    }

    /// Number of layers currently registered.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }
}

impl Default for PatternComposer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gradient::{Gradient, GradientStop};

    fn grey_gradient() -> Gradient {
        Gradient::new(vec![
            GradientStop::new(0.0, [0, 0, 0]),
            GradientStop::new(1.0, [255, 255, 255]),
        ])
    }

    // BlendMode tests
    #[test]
    fn test_add_clamps() {
        assert!((BlendMode::Add.apply(0.8, 0.5) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_add_normal() {
        assert!((BlendMode::Add.apply(0.3, 0.2) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_multiply_zero() {
        assert_eq!(BlendMode::Multiply.apply(0.5, 0.0), 0.0);
    }

    #[test]
    fn test_multiply_identity() {
        assert!((BlendMode::Multiply.apply(0.6, 1.0) - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_screen_full() {
        // screen(1, 1) = 1
        assert!((BlendMode::Screen.apply(1.0, 1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_screen_zero() {
        assert!((BlendMode::Screen.apply(0.0, 0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_screen_brightens() {
        let s = BlendMode::Screen.apply(0.5, 0.5);
        assert!(
            s > 0.5,
            "screen should be brighter than either input: {}",
            s
        );
    }

    #[test]
    fn test_overlay_dark() {
        // base < 0.5: 2 * 0.3 * 0.4 = 0.24
        let out = BlendMode::Overlay.apply(0.3, 0.4);
        assert!((out - 0.24).abs() < 1e-6, "overlay dark: {}", out);
    }

    #[test]
    fn test_overlay_bright() {
        // base >= 0.5: 1 - 2*(1-0.7)*(1-0.8) = 1 - 2*0.3*0.2 = 0.88
        let out = BlendMode::Overlay.apply(0.7, 0.8);
        assert!((out - 0.88).abs() < 1e-6, "overlay bright: {}", out);
    }

    #[test]
    fn test_difference_symmetric() {
        let a = BlendMode::Difference.apply(0.8, 0.3);
        let b = BlendMode::Difference.apply(0.3, 0.8);
        assert!((a - b).abs() < 1e-6);
    }

    #[test]
    fn test_difference_same() {
        assert!((BlendMode::Difference.apply(0.5, 0.5) - 0.0).abs() < 1e-6);
    }

    // PatternComposer tests
    #[test]
    fn test_empty_composer_all_zeros() {
        let comp = PatternComposer::new();
        let pixels = comp.render(4, 4);
        assert!(pixels.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_render_size() {
        let comp = PatternComposer::new();
        let pixels = comp.render(8, 6);
        assert_eq!(pixels.len(), 48);
    }

    #[test]
    fn test_single_constant_layer() {
        let mut comp = PatternComposer::new();
        comp.add_layer(Layer::new(|_, _| 1.0, 1.0, BlendMode::Add));
        let pixels = comp.render(4, 4);
        assert!(pixels.iter().all(|&v| (v - 1.0).abs() < 1e-6));
    }

    #[test]
    fn test_layer_weight_scales() {
        let mut comp = PatternComposer::new();
        comp.add_layer(Layer::new(|_, _| 1.0, 0.5, BlendMode::Add));
        let pixels = comp.render(2, 2);
        assert!(pixels.iter().all(|&v| (v - 0.5).abs() < 1e-6));
    }

    #[test]
    fn test_render_rgb_length() {
        let comp = PatternComposer::new();
        let g = grey_gradient();
        let rgb = comp.render_rgb(4, 4, &g);
        assert_eq!(rgb.len(), 16);
    }

    #[test]
    fn test_render_rgb_black_on_empty() {
        let comp = PatternComposer::new();
        let g = grey_gradient();
        let rgb = comp.render_rgb(2, 2, &g);
        for px in &rgb {
            assert_eq!(*px, [0u8, 0u8, 0u8]);
        }
    }

    #[test]
    fn test_layer_count() {
        let mut comp = PatternComposer::new();
        assert_eq!(comp.layer_count(), 0);
        comp.add_layer(Layer::new(|_, _| 0.5, 1.0, BlendMode::Multiply));
        assert_eq!(comp.layer_count(), 1);
    }

    #[test]
    fn test_gradient_x_pattern() {
        // Pattern that returns x-coord; leftmost pixel should be darker.
        let mut comp = PatternComposer::new();
        comp.add_layer(Layer::new(|x, _y| x, 1.0, BlendMode::Add));
        let g = grey_gradient();
        let rgb = comp.render_rgb(10, 1, &g);
        // First pixel (x=0) darker than last (x=1)
        assert!(rgb[0][0] < rgb[9][0]);
    }

    #[test]
    fn test_multiply_two_layers() {
        // layer1 = 1.0 constant (Add), layer2 = 0.5 constant (Multiply)
        let mut comp = PatternComposer::new();
        comp.add_layer(Layer::new(|_, _| 1.0, 1.0, BlendMode::Add));
        comp.add_layer(Layer::new(|_, _| 1.0, 0.5, BlendMode::Multiply));
        let pixels = comp.render(2, 2);
        // acc after layer1 = 1.0; after Multiply(1.0, 0.5) = 0.5
        assert!(pixels.iter().all(|&v| (v - 0.5).abs() < 1e-6));
    }
}
