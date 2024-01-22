use std::f32::consts::PI;

use glam::Mat4;

/**
 * Graphical camera which controls what can be seen and from what perspective.
 */
pub struct Camera {
    pub target: CameraTarget,
    pub projection: Mat4,
    pub previous_projection: Mat4,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: CameraTarget::OnScreen,
            projection: Mat4::IDENTITY,
            previous_projection: Mat4::IDENTITY,
        }
    }
}

impl Camera {

    pub fn new(projection: Mat4) -> Self {
        Self {
            target: CameraTarget::OnScreen,
            projection,
            previous_projection: Mat4::IDENTITY,
        }
    }

    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let projection = Mat4::orthographic_lh(left, right, bottom, top, near, far);
        Self {
            target: CameraTarget::OnScreen,
            projection,
            previous_projection: Mat4::IDENTITY,
        }
    }

    /**
     * Perspective camera. FOV is in degrees.
     */
    pub fn perspective(fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        let fov = fov * PI / 180.0;
        let projection = Mat4::perspective_lh(fov, aspect_ratio, near, far);
        Self {
            target: CameraTarget::OnScreen,
            projection,
            previous_projection: Mat4::IDENTITY,
        }
    }
}

/**
 * Which texture to render to.
 */
pub enum CameraTarget {
    OnScreen,
    OffScreen,
}