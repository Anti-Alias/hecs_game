use glam::{Vec3, Quat, Affine3A, Mat4};


/**
 * The 3D transformation of an object, which includes its translation (position) rotation and scale.
 */
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl From<Transform> for Affine3A {
    fn from(transform: Transform) -> Self {
        Self::from_scale_rotation_translation(
            transform.scale,
            transform.rotation,
            transform.translation
        )
    }
}

impl From<Transform> for Mat4 {
    fn from(transform: Transform) -> Self {
        Self::from_scale_rotation_translation(
            transform.scale,
            transform.rotation,
            transform.translation
        )
    }
}