use glam::{Vec3, Quat, Affine3A, Mat4, EulerRot};


/**
 * The 3D transformation of an object, which includes its translation (position) rotation and scale.
 */
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::new(0.0, 0.0, 0.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::new(1.0, 1.0, 1.0),
    };

    pub fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    pub fn with_xyz(mut self, x: f32, y: f32, z: f32) -> Self {
        self.translation = Vec3::new(x, y, z);
        self
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_scale_xyz(mut self, x: f32, y: f32, z: f32) -> Self {
        self.scale = Vec3::new(x, y, z);
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_euler(mut self, rot: EulerRot, a: f32, b: f32, c: f32) -> Self {
        self.rotation = Quat::from_euler(rot, a, b, c);
        self
    }
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