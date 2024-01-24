use hecs::World;
use winit::keyboard::KeyCode;
use crate::math::Transform;
use crate::{Game, Keyboard, Plugin, RunContext, Stage};

pub struct FlycamPlugin;
impl Plugin for FlycamPlugin {
    fn install(&mut self, builder: &mut crate::AppBuilder) {
        builder.system(Stage::Update, control_flycam);
    }
}


fn control_flycam(game: &mut Game, ctx: RunContext) {

    let mut world = game.get::<&mut World>();
    let keyboard = game.get::<&Keyboard>();
    let delta = ctx.delta_secs();
    
    for (_, (transform, flycam)) in world.query_mut::<(&mut Transform, &Flycam)>() {
        if keyboard.is_pressed(KeyCode::KeyA) {
            transform.translation.x -= flycam.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::KeyD) {
            transform.translation.x += flycam.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::KeyW) {
            transform.translation.z -= flycam.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::KeyS) {
            transform.translation.z += flycam.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::Space) {
            transform.translation.y += flycam.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::ShiftLeft) {
            transform.translation.y -= flycam.speed * delta;
        }
    }
}


pub struct Flycam {
    pub speed: f32
}

impl Default for Flycam {
    fn default() -> Self {
        Self {
            speed: 2.0,
        }
    }
}