//! Ring-buffer trail storage for geodesic curves.
//!
//! Each active geodesic owns one [`TrailBuffer`] that records its most recent
//! 3D positions in a fixed-capacity ring buffer. When the buffer is full, the
//! oldest entry is overwritten. Alpha is computed per-vertex at read time so
//! the oldest samples are transparent and the newest are fully opaque.

use bytemuck::{Pod, Zeroable};

/// A single vertex in a rendered trail.
///
/// Stores the 3D world-space position and an RGBA colour. The alpha channel
/// is computed by [`TrailBuffer::ordered_vertices`] to produce a quadratic
/// fade from transparent (tail) to opaque (head).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct TrailVertex {
    /// World-space position of this trail sample.
    pub position: [f32; 3],
    /// RGBA colour including fade alpha.
    pub color: [f32; 4],
}

/// Fixed-capacity ring buffer storing [`TrailVertex`] samples for one geodesic.
///
/// New positions are appended via [`TrailBuffer::push`]. When the buffer is
/// full the oldest sample is silently overwritten. Use
/// [`TrailBuffer::ordered_vertices`] to read samples in chronological order
/// with fade alpha applied.
pub struct TrailBuffer {
    /// Backing store; always allocated at full capacity.
    pub vertices: Vec<TrailVertex>,
    /// Index of the next write slot.
    pub head: usize,
    /// Number of valid samples currently stored (saturates at `capacity`).
    pub count: usize,
    /// Maximum number of samples stored (allocated capacity).
    pub capacity: usize,
    /// Base colour applied to every vertex before the fade alpha is computed.
    pub color: [f32; 4],
    /// Exponent for the alpha fade curve (`1.0` = linear, `2.0` = quadratic).
    pub fade_power: f32,
}

impl TrailBuffer {
    /// Allocate a new trail buffer with the given `capacity`, `color`, and `fade_power`.
    ///
    /// `fade_power` controls the alpha fade curve: `1.0` is linear, `2.0` is
    /// quadratic (the original behaviour), `3.0` is cubic, etc.
    ///
    /// All slots are initialised to the zero position and zero alpha.
    pub fn new(capacity: usize, color: [f32; 4], fade_power: f32) -> Self {
        // Guard against zero capacity to avoid divide-by-zero in ordered_vertices.
        let capacity = capacity.max(1);
        let fade_power = if fade_power <= 0.0 { 2.0 } else { fade_power };
        Self {
            vertices: vec![
                TrailVertex {
                    position: [0.0; 3],
                    color: [0.0; 4]
                };
                capacity
            ],
            head: 0,
            count: 0,
            capacity,
            color,
            fade_power,
        }
    }

    /// Append a new 3D position sample to the trail.
    ///
    /// If the buffer is full the oldest sample is overwritten.
    pub fn push(&mut self, pos: [f32; 3]) {
        self.vertices[self.head] = TrailVertex {
            position: pos,
            color: self.color,
        };
        self.head = (self.head + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    /// Reset the buffer, discarding all stored samples.
    pub fn clear(&mut self) {
        self.count = 0;
        self.head = 0;
    }

    /// Return all stored samples in chronological order (oldest first) with
    /// fade alpha applied.
    ///
    /// Alpha is `(i / count)^fade_power` where `i = 0` is the oldest sample
    /// and `i = count - 1` is the newest, making the tail transparent and the
    /// head fully opaque. Allocates a new `Vec`; the render loop uses
    /// [`TrailBuffer::append_ordered`] instead to reuse one buffer.
    pub fn ordered_vertices(&self) -> Vec<TrailVertex> {
        let mut out = Vec::with_capacity(self.count);
        self.append_ordered(&mut out, None);
        out
    }

    /// Append the samples in chronological order with fade alpha to `out`,
    /// optionally replacing the RGB of every vertex with `rgb`.
    ///
    /// Same result as [`TrailBuffer::ordered_vertices`] but without a
    /// per-call allocation, without a modulo per vertex, and with the common
    /// `fade_power == 2.0` (and `1.0`) cases computed without `powf`.
    pub fn append_ordered(&self, out: &mut Vec<TrailVertex>, rgb: Option<[f32; 3]>) {
        let n = self.count;
        if n == 0 {
            return;
        }
        // Oldest sample first: when full, the oldest is at `head`.
        let (older, newer) = if n == self.capacity {
            let (a, b) = self.vertices.split_at(self.head);
            (b, a)
        } else {
            (&self.vertices[..n], &self.vertices[..0])
        };
        let inv = 1.0 / n as f32;
        let power = self.fade_power;
        let fade = |i: usize| -> f32 {
            let x = i as f32 * inv;
            if power == 2.0 {
                x * x
            } else if power == 1.0 {
                x
            } else {
                x.powf(power)
            }
        };
        out.reserve(n);
        for (i, v) in older.iter().chain(newer.iter()).enumerate() {
            let [r, g, b] = rgb.unwrap_or([v.color[0], v.color[1], v.color[2]]);
            out.push(TrailVertex {
                position: v.position,
                color: [r, g, b, fade(i)],
            });
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn red() -> [f32; 4] {
        [1.0, 0.0, 0.0, 1.0]
    }

    /// A freshly created buffer should report zero count.
    #[test]
    fn new_buffer_is_empty() {
        let buf = TrailBuffer::new(10, red(), 2.0);
        assert_eq!(buf.count, 0);
        assert_eq!(buf.ordered_vertices().len(), 0);
    }

    /// After filling the buffer beyond capacity the count saturates.
    #[test]
    fn count_saturates_at_capacity() {
        let cap = 5;
        let mut buf = TrailBuffer::new(cap, red(), 2.0);
        for i in 0..20 {
            buf.push([i as f32, 0.0, 0.0]);
        }
        assert_eq!(buf.count, cap);
        assert_eq!(buf.ordered_vertices().len(), cap);
    }

    /// After clear() the buffer should behave as if newly created.
    #[test]
    fn clear_resets_buffer() {
        let mut buf = TrailBuffer::new(8, red(), 2.0);
        for i in 0..8 {
            buf.push([i as f32, 0.0, 0.0]);
        }
        buf.clear();
        assert_eq!(buf.count, 0);
        assert_eq!(buf.ordered_vertices().len(), 0);
    }

    /// The oldest vertex should have the lowest alpha (near zero) and the
    /// newest should have the highest alpha (near 1.0) after quadratic fade.
    #[test]
    fn fade_increases_from_tail_to_head() {
        let cap = 10;
        let mut buf = TrailBuffer::new(cap, red(), 2.0);
        for i in 0..cap {
            buf.push([i as f32, 0.0, 0.0]);
        }
        let verts = buf.ordered_vertices();
        // Oldest vertex (index 0) has the lowest alpha.
        let first_alpha = verts[0].color[3];
        let last_alpha = verts[cap - 1].color[3];
        assert!(
            first_alpha < last_alpha,
            "expected first_alpha < last_alpha, got {first_alpha} vs {last_alpha}"
        );
    }

    /// When the buffer wraps around, ordered_vertices must still return samples
    /// in insertion order: oldest first.
    #[test]
    fn ring_wrap_preserves_order() {
        let cap = 4;
        let mut buf = TrailBuffer::new(cap, red(), 2.0);
        // Fill then overshoot by 2.
        for i in 0..6u32 {
            buf.push([i as f32, 0.0, 0.0]);
        }
        // The last 4 inserted values are [2, 3, 4, 5].
        let verts = buf.ordered_vertices();
        assert_eq!(verts.len(), cap);
        let positions: Vec<f32> = verts.iter().map(|v| v.position[0]).collect();
        assert_eq!(
            positions,
            vec![2.0, 3.0, 4.0, 5.0],
            "ring wrap order incorrect: {positions:?}"
        );
    }

    /// A zero-capacity guard: TrailBuffer::new clamps capacity to 1 to avoid
    /// divide-by-zero in ordered_vertices.
    #[test]
    fn zero_capacity_is_clamped_to_one() {
        let buf = TrailBuffer::new(0, red(), 2.0);
        assert_eq!(buf.capacity, 1);
    }

    /// Linear fade (power=1) should produce linearly increasing alpha values.
    #[test]
    fn linear_fade_power() {
        let cap = 4;
        let mut buf = TrailBuffer::new(cap, red(), 1.0);
        for i in 0..cap {
            buf.push([i as f32, 0.0, 0.0]);
        }
        let verts = buf.ordered_vertices();
        // With linear fade and 4 samples: alphas should be 0/4, 1/4, 2/4, 3/4.
        let expected = [0.0f32, 0.25, 0.5, 0.75];
        for (i, (v, &e)) in verts.iter().zip(expected.iter()).enumerate() {
            assert!(
                (v.color[3] - e).abs() < 1e-5,
                "linear fade alpha[{i}] expected {e}, got {}",
                v.color[3]
            );
        }
    }

    /// Cubic fade (power=3) should produce more aggressive tail fade than quadratic.
    #[test]
    fn cubic_fade_is_more_aggressive_than_quadratic() {
        let cap = 10;
        let mut buf2 = TrailBuffer::new(cap, red(), 2.0);
        let mut buf3 = TrailBuffer::new(cap, red(), 3.0);
        for i in 0..cap {
            buf2.push([i as f32, 0.0, 0.0]);
            buf3.push([i as f32, 0.0, 0.0]);
        }
        let v2 = buf2.ordered_vertices();
        let v3 = buf3.ordered_vertices();
        // For the middle vertex, cubic should be darker (lower alpha) than quadratic.
        let mid = cap / 2;
        assert!(
            v3[mid].color[3] <= v2[mid].color[3],
            "cubic alpha ({}) should be <= quadratic alpha ({}) at midpoint",
            v3[mid].color[3],
            v2[mid].color[3]
        );
    }

    /// The allocation-free path gives exactly the same vertices as the old
    /// modulo-and-powf loop, for partial and wrapped buffers and any power.
    #[test]
    fn append_ordered_matches_reference() {
        fn reference(t: &TrailBuffer) -> Vec<([f32; 3], [f32; 4])> {
            (0..t.count)
                .map(|i| {
                    let alpha = (i as f32 / t.count.max(1) as f32).powf(t.fade_power);
                    let idx = if t.count == t.capacity {
                        (t.head + i) % t.capacity
                    } else {
                        i
                    };
                    let v = t.vertices[idx];
                    (v.position, [v.color[0], v.color[1], v.color[2], alpha])
                })
                .collect()
        }
        for power in [1.0f32, 2.0, 2.5, 3.0] {
            for pushes in [0usize, 1, 4, 7, 8, 9, 23] {
                let mut t = TrailBuffer::new(8, [0.2, 0.4, 0.6, 1.0], power);
                for k in 0..pushes {
                    t.push([k as f32, -(k as f32), 0.5]);
                }
                let got: Vec<_> = t
                    .ordered_vertices()
                    .iter()
                    .map(|v| (v.position, v.color))
                    .collect();
                let want = reference(&t);
                assert_eq!(got.len(), want.len());
                for (g, w) in got.iter().zip(&want) {
                    assert_eq!(g.0, w.0);
                    for c in 0..4 {
                        assert!(
                            (g.1[c] - w.1[c]).abs() < 1e-6,
                            "power {power} pushes {pushes}"
                        );
                    }
                }
                let mut buf = vec![];
                t.append_ordered(&mut buf, Some([1.0, 0.0, 0.0]));
                assert!(buf.iter().all(|v| v.color[..3] == [1.0, 0.0, 0.0]));
            }
        }
    }
}
