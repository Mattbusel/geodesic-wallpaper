use glam::{Mat4, Vec3};

pub struct Camera {
    pub angle: f32,
    pub elevation: f32,
    pub distance: f32,
    pub fov_y: f32,
    pub aspect: f32,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            angle: 0.0,
            elevation: 0.4,
            distance: 6.0,
            fov_y: 0.8,
            aspect,
        }
    }

    pub fn orbit(&mut self, delta_angle: f32) {
        self.angle += delta_angle;
    }

    pub fn view_proj(&self) -> Mat4 {
        let eye = Vec3::new(
            self.distance * self.elevation.cos() * self.angle.cos(),
            self.distance * self.elevation.cos() * self.angle.sin(),
            self.distance * self.elevation.sin(),
        );
        let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Z);
        let proj = Mat4::perspective_rh(self.fov_y, self.aspect, 0.1, 50.0);
        proj * view
    }
}
