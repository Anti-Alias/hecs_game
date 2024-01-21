use glam::Mat4;

pub struct Camera {
    pub target: CameraTarget,
    pub projection: Projection,
}

pub enum CameraTarget {
    OnScreen,
    OffScreen,
}

/**
 * Either an orthographic or perspective camera projection.
 */
pub enum Projection {
    Orthographic(OrthographicProjection),
    Perspective(PerspectiveProjection),
}

impl Projection {
    pub fn matrix(&self) -> Mat4 {
        match self {
            Self::Orthographic(ortho) => Mat4::orthographic_lh(
                ortho.left,
                ortho.right,
                ortho.bottom,
                ortho.top,
                ortho.near,
                ortho.far
            ),
            Self::Perspective(persp) => Mat4::perspective_lh(
                persp.fov,
                persp.aspect_ratio,
                persp.near,
                persp.far
            ),
        }
    }
}

pub struct OrthographicProjection {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

pub struct PerspectiveProjection {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}