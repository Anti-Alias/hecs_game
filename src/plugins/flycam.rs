use glam::{Quat, Vec3};
use hecs::World;
use winit::keyboard::KeyCode;
use crate::math::Transform;
use crate::{Cursor, Game, Keyboard, Plugin, RunContext, Stage};

pub struct FlycamPlugin;
impl Plugin for FlycamPlugin {
    fn install(&mut self, builder: &mut crate::AppBuilder) {
        builder.system(Stage::Update, control_flycam);
    }
}


fn control_flycam(game: &mut Game, ctx: RunContext) {
    let mut world       = game.get::<&mut World>();
    let keyboard        = game.get::<&Keyboard>();
    let cursor          = game.get::<&Cursor>();
    let delta           = ctx.delta_secs();
    
    let cursor_movement = cursor.movement();
    for (_, (transform, flycam)) in world.query_mut::<(&mut Transform, &mut Flycam)>() {

        // Rotates using mouse
        flycam.yaw -= cursor_movement.x * flycam.sensitivity * SENSITIVITY_SCALE;
        flycam.pitch -= cursor_movement.y * flycam.sensitivity * SENSITIVITY_SCALE;

        // Translates using keys.
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
        transform.rotation = flycam.rotation()
    }
}

const SENSITIVITY_SCALE: f32 = 0.005;

pub struct Flycam {
    pub speed: f32,
    pub sensitivity: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Default for Flycam {
    fn default() -> Self {
        Self {
            speed: 2.0,
            sensitivity: 1.0,
            pitch: 0.0,
            yaw: 0.0,
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