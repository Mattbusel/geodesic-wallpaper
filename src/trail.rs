use bytemuck::{Pod, Zeroable};

/// A single vertex in a trail: 3D world position + color (RGBA) + fade alpha
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct TrailVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

/// Ring buffer of trail vertices for one geodesic
pub struct TrailBuffer {
    pub vertices: Vec<TrailVertex>,
    pub head: usize,
    pub count: usize,
    pub capacity: usize,
    pub color: [f32; 4],
}

impl TrailBuffer {
    pub fn new(capacity: usize, color: [f32; 4]) -> Self {
        Self {
            vertices: vec![TrailVertex { position: [0.0; 3], color: [0.0; 4] }; capacity],
            head: 0,
            count: 0,
            capacity,
            color,
        }
    }

    pub fn push(&mut self, pos: [f32; 3]) {
        self.vertices[self.head] = TrailVertex {
            position: pos,
            color: self.color,
        };
        self.head = (self.head + 1) % self.capacity;
        if self.count < self.capacity { self.count += 1; }
    }

    pub fn clear(&mut self) {
        self.count = 0;
        self.head = 0;
    }

    /// Returns vertices in order from oldest to newest, with fading alpha applied
    pub fn ordered_vertices(&self) -> Vec<TrailVertex> {
        let mut out = Vec::with_capacity(self.count);
        for i in 0..self.count {
            let age_frac = i as f32 / self.count.max(1) as f32; // 0 = oldest, 1 = newest
            let alpha = age_frac * age_frac; // quadratic fade
            let idx = if self.count == self.capacity {
                (self.head + i) % self.capacity
            } else {
                i
            };
            let v = self.vertices[idx];
            out.push(TrailVertex {
                position: v.position,
                color: [v.color[0], v.color[1], v.color[2], alpha],
            });
        }
        out
    }
}
