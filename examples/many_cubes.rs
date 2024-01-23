use std::f32::consts::TAU;
use glam::{Vec3, Quat};
use hecs_game::math::Transform;
use hecs_game::{g3d, App, ClientPlugin, AppBuilder, Color, Handle, GraphicsState, SceneGraph, Stage, RunContext, Game, Projection, Keyboard, StartEvent};
use hecs::World;
use rand::{SeedableRng, Rng};
use rand::rngs::SmallRng;
use winit::keyboard::KeyCode;

fn main() {
    env_logger::init();
    let mut builder = App::builder();
    builder
        .plugin(ClientPlugin)
        .plugin(plugin);
    builder.run();
}

fn plugin(builder: &mut AppBuilder) {
    builder
        .system(Stage::Update, rotate_cubes)
        .system(Stage::Update, control_flycam)
        .event_handler(handle_start)
        .tick_rate(60.0);
}

fn handle_start(game: &mut Game, _event: &StartEvent) {
    
    // Extracts domains
    let (mut world, state, mut scene) = game.all::<(
        &mut World,
        &GraphicsState,
        &mut SceneGraph<g3d::Renderable>
    )>();

    // Spawns camera
    let cam_tracker = scene.insert(g3d::Renderable::camera());
    let cam_transform = Transform::default().with_xyz(0.0, 0.0, 1.0);
    let cam_projection = Projection::perspective(90.0, 1.0, 0.1, 1000.0);
    world.spawn((cam_tracker, cam_transform, cam_projection, Flycam::default()));
    
    // Creates material
    let material: g3d::Material = Color::BLUE.into();
    let material = g3d::GpuMaterial::from_material(&material, &state.device);
    let material = Handle::new(material);

    // Creates blue mesh
    let blue_mesh: g3d::Mesh = g3d::Mesh::from(g3d::Cuboid {
        center: Vec3::new(0.0, 0.0, 0.0),
        half_extents: Vec3::new(0.5, 0.5, 0.5),
        color: Color::BLUE,
    });
    let blue_mesh = g3d::GpuMesh::from_mesh(&blue_mesh, &state.device);
    let blue_mesh = Handle::new(blue_mesh);

    // Creates red mesh
    let red_mesh: g3d::Mesh = g3d::Mesh::from(g3d::Cuboid {
        center: Vec3::new(0.0, 0.0, 0.0),
        half_extents: Vec3::new(0.5, 0.5, 0.5),
        color: Color::RED,
    });
    let red_mesh = g3d::GpuMesh::from_mesh(&red_mesh, &state.device);
    let red_mesh = Handle::new(red_mesh);
    
    // Spawns cubes
    let mut rng = SmallRng::seed_from_u64(48);
    for _ in 0..5000 {

        // Creates random transform
        let scale = 0.2 + rng.gen::<f32>() * 0.2;
        let transform = Transform::IDENTITY
            .with_translation(Vec3::new(
                rng.gen::<f32>() * 2.0 - 1.0,
                rng.gen::<f32>() * 2.0 - 1.0,
                -1.0,
            ))
            .with_scale(Vec3::splat(scale));

        // Creates random rotator component
        let rotator = Rotator {
            axis: Vec3::new(
                rng.gen::<f32>() * 2.0 - 1.0,
                rng.gen::<f32>() * 2.0 - 1.0,
                rng.gen::<f32>() * 2.0 - 1.0,
            ).normalize(),
            angle: rng.gen::<f32>() * TAU,
            speed: rng.gen::<f32>(),
        };

        // Selects random mesh
        let mesh = match rng.gen::<bool>() {
            true => blue_mesh.clone(),
            false => red_mesh.clone(),
        };

        // Spawns cube with above data
        let renderable = g3d::Renderable::empty()
            .with_mat_mesh(material.clone(), mesh)
            .with_aabb_volume(Vec3::ZERO, Vec3::splat(0.5));
        let renderable = scene.insert(renderable);
        world.spawn((renderable, transform, rotator));
    }
}

fn rotate_cubes(game: &mut Game, ctx: RunContext) {
    let mut world = game.get::<&mut World>();
    for (_, (transform, rotator)) in world.query_mut::<(&mut Transform, &mut Rotator)>() {
        transform.rotation = Quat::from_axis_angle(rotator.axis, rotator.angle);
        rotator.angle += rotator.speed * ctx.delta_secs();
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

struct Rotator {
    axis: Vec3,
    angle: f32,
    speed: f32,
}


struct Flycam {
    pub speed: f32
}

impl Default for Flycam {
    fn default() -> Self {
        Self {
            speed: 2.0,
        }
    }
}