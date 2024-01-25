use std::f32::consts::PI;
use glam::Mat4;

/// Camera projection component.
pub struct Projection(pub Mat4);
impl Default for Projection {
    fn default() -> Self {
        Self::orthographic(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0)
    }
}

impl Projection {

    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        Self(Mat4::orthographic_rh(left, right, bottom, top, near, far))
    }

    /// Perspective camera. FOV is in degrees.
    pub fn perspective(fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        let fov = fov * PI / 180.0;
        Self(Mat4::perspective_rh(fov, aspect_ratio, near, far))
    }
}