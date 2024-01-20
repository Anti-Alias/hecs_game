use glam::Mat4;

pub struct Camera {
    pub target: CameraTarget,
    pub projection: Projection,
}

pub enum CameraTarget {
    OnScreen,
    OffScreen,
}

pub enum Projection {
    Orthographic(OrthographicCamera),
    Perspective(PerspectiveCamera),
}

impl Projection {
    pub fn projection_view(&self) -> Mat4 {
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

pub struct OrthographicCamera {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

pub struct PerspectiveCamera {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}