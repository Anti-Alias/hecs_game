use std::f32::consts::PI;

use glam::{Mat4, Quat, Vec3};
use hecs::World;
use winit::keyboard::KeyCode;
use crate::math::{lerp_matrices, Transform};
use crate::{Cursor, Game, Keyboard, Plugin, Projection, RunContext, Stage};

pub struct FlycamPlugin;
impl Plugin for FlycamPlugin {
    fn install(&mut self, builder: &mut crate::AppBuilder) {
        builder.system(Stage::PostUpdate, run_cameras);
    }
}


fn run_cameras(game: &mut Game, ctx: RunContext) {
    
    let mut world       = game.get::<&mut World>();
    let keyboard        = game.get::<&Keyboard>();
    let cursor          = game.get::<&Cursor>();
    let delta           = ctx.delta_secs();
    let cursor_movement = cursor.movement();

    // Applies projectors to projections
    for (_, (projector, projection)) in world.query_mut::<(&OrthographicProjector, &mut Projection)>() {
        projection.0 = projector.compute_projection();
    }
    for (_, (projector, projection)) in world.query_mut::<(&PerspectiveProjector, &mut Projection)>() {
        projection.0 = projector.compute_projection();
    }
    for (_, (projector, projection)) in world.query_mut::<(&DualProjector, &mut Projection)>() {
        projection.0 = projector.compute_projection();
    }

    // Moves flycams
    for (_, (transform, flycam)) in world.query_mut::<(&mut Transform, &mut Flycam)>() {

        flycam.yaw -= cursor_movement.x * flycam.sensitivity * SENSITIVITY_SCALE;
        flycam.pitch -= cursor_movement.y * flycam.sensitivity * SENSITIVITY_SCALE;
        transform.rotation = flycam.rotation();

        let (right, up, forward) = flycam.axes();
        if keyboard.is_pressed(KeyCode::KeyA) {
            transform.translation -= right * flycam.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::KeyD) {
            transform.translation += right * flycam.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::KeyW) {
            transform.translation += forward * flycam.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::KeyS) {
            transform.translation -= forward * flycam.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::Space) {
            transform.translation += up * flycam.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::ShiftLeft) {
            transform.translation -= up * flycam.speed * delta;
        }
    }
}

const SENSITIVITY_SCALE: f32 = 0.005;

pub struct Flycam {
    pub speed: f32,
    pub sensitivity: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub projector: DualProjector,
}

impl Default for Flycam {
    fn default() -> Self {
        Self {
            speed: 2.0,
            sensitivity: 1.0,
            pitch: 0.0,
            yaw: 0.0,
            projector: DualProjector::default(),
        }
    }
}

impl Flycam {
    pub fn rotation(&self) -> Quat {
        Quat::IDENTITY * Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(self.pitch)
    }

    pub fn axes(&self) -> (Vec3, Vec3, Vec3) {
        let rotation = self.rotation();
        (
            rotation * Vec3::X,
            rotation * Vec3::Y,
            rotation * Vec3::NEG_Z,
        )
    }
}

/// Component that manipulates a camera's projection matrix.
/// Has two projections, orthographic and perspective.
/// Can interpolate between both for neat effects.
#[derive(Clone, Default, Debug)]
pub struct DualProjector {
    pub orthographic: OrthographicProjector,
    pub perspective: PerspectiveProjector,
    pub t: f32,
}

impl DualProjector {

    /// Interpolated orthographic and perspective projections.
    /// t of 0.0 = orthographic
    /// t of 1.0 = perspective
    pub fn compute_projection(&self) -> Mat4 {
        let ortho_proj = self.orthographic.compute_projection();
        let persp_proj = self.perspective.compute_projection();
        lerp_matrices(ortho_proj, persp_proj, self.t)
    }
}

#[derive(Clone, Debug)]
pub struct OrthographicProjector {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

impl OrthographicProjector {
    pub fn compute_projection(&self) -> Mat4 {
        Mat4::orthographic_rh(self.left, self.right, self.bottom, self.top, self.near, self.far)
    }
}

impl Default for OrthographicProjector {
    fn default() -> Self {
        Self {
            left: -1.0,
            right: 1.0,
            bottom: -1.0,
            top: 1.0,
            near: 0.0,
            far: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PerspectiveProjector {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl PerspectiveProjector {
    pub fn compute_projection(&self) -> Mat4 {
        let fov = self.fov * PI / 180.0;
        Mat4::perspective_rh(fov, self.aspect_ratio, self.near, self.far)
    }
}

impl Default for PerspectiveProjector {
    fn default() -> Self {
         Self {
            fov: 80.0,
            aspect_ratio: 16.0 / 9.0,
            near: 0.1,
            far: 10000.0,
        }
    }
}