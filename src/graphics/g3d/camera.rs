use glam::Mat4;

/**
 * Graphical camera which controls what can be seen and from what perspective.
 */
pub struct Camera {
    pub target: CameraTarget,
    pub projection: Mat4,
}

impl Camera {

    pub fn new(projection: Mat4) -> Self {
        Self {
            target: CameraTarget::OnScreen,
            projection,
        }
    }

    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let projection = Mat4::orthographic_rh(left, right, bottom, top, near, far);
        Self {
            target: CameraTarget::OnScreen,
            projection,
        }
    }

    pub fn perspective(fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        let projection = Mat4::perspective_rh(fov, aspect_ratio, near, far);
        Self {
            target: CameraTarget::OnScreen,
            projection,
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