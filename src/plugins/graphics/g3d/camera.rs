use std::f32::consts::PI;
use glam::Mat4;
use crate::{InterpolationMode, Rect};

/**
 * Graphical camera which controls what can be seen and from what perspective.
 */
pub struct Camera {
    pub target: CameraTarget,
    pub(crate) projection: Mat4,
    pub(crate) previous_projection: Mat4,
    pub(crate) viewport: Option<Rect>,
    pub interpolation_mode: InterpolationMode,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: CameraTarget::OnScreen,
            projection: Mat4::IDENTITY,
            previous_projection: Mat4::IDENTITY,
            interpolation_mode: InterpolationMode::Skip,
            viewport: None,
        }
    }
}

impl Camera {

    pub fn new(projection: Mat4) -> Self {
        Self {
            target: CameraTarget::OnScreen,
            projection,
            ..Default::default()
        }
    }

    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let projection = Mat4::orthographic_lh(left, right, bottom, top, near, far);
        Self {
            target: CameraTarget::OnScreen,
            projection,
            ..Default::default()
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
            ..Default::default()
        }
    }

    pub fn projection(&self) -> Mat4 {
        self.projection
    }

    pub fn set_projection(&mut self, projection: Mat4) {
        match self.interpolation_mode {
            InterpolationMode::Interpolate => {
                self.previous_projection = self.projection;
                self.projection = projection;
            },
            InterpolationMode::Skip => {
                self.previous_projection = projection;
                self.projection = projection;
                self.interpolation_mode = InterpolationMode::Interpolate;
            },
            InterpolationMode::None => {
                self.previous_projection = projection;
                self.projection = projection;
            },
        }
    }

    pub fn with_interpolation_mode(mut self, interpolation_mode: InterpolationMode) -> Self {
        self.interpolation_mode = interpolation_mode;
        self
    }
}

/**
 * Which texture to render to.
 */
pub enum CameraTarget {
    OnScreen,
    OffScreen,
}