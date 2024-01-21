use std::f32::consts::TAU;
use glam::{Vec3, Quat};
use hecs_game::g3d::Camera;
use hecs_game::math::Transform;
use hecs_game::{g3d, App, ClientPlugin, AppBuilder, Color, Handle, GraphicsState, SceneGraph, Stage, RunContext, Game};
use hecs::World;
use rand::{SeedableRng, Rng};
use rand::rngs::SmallRng;

fn main() {
    env_logger::init();
    let mut builder = App::builder();
    builder
        .plugin(ClientPlugin)
        .plugin(plugin);
    builder.run();
}

fn plugin(builder: &mut AppBuilder) {

    builder.add_system(Stage::Update, rotate_cubes, true);

    // Extracts domains
    let (mut world, state, mut scene) = builder
        .game()
        .all::<(&mut World, &GraphicsState, &mut SceneGraph<g3d::Renderable>)>();

    // Spawns camera
    //let camera = Camera::perspective(80.0, state.aspect_ratio(), 0.1, 1000.0);
    let camera = Camera::orthographic(-1.0, 1.0, -1.0, 1.0, 0.0, 100.0);
    let camera = g3d::Renderable::camera(camera);
    let cam_tracker = scene.insert(camera);
    let cam_transform = Transform::IDENTITY;
    world.spawn((cam_tracker, cam_transform));
    
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
    for _ in 0..10 {

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
            speed: rng.gen::<f32>() * 0.1,
        };

        // Selects random mesh handle
        let mesh_flag: bool = rng.gen();
        let mesh = match mesh_flag {
            true => blue_mesh.clone(),
            false => red_mesh.clone(),
        };

        // Spawns cube with above data
        let renderable = g3d::Renderable::mat_mesh(material.clone(), mesh);
        let renderable = scene.insert(renderable);
        world.spawn((renderable, transform, rotator));
    }
}

fn rotate_cubes(game: &mut Game, _ctx: RunContext) {
    let mut world = game.get::<&mut World>();
    for (_, (transform, rotator)) in world.query_mut::<(&mut Transform, &mut Rotator)>() {
        transform.rotation = Quat::from_axis_angle(rotator.axis, rotator.angle);
        rotator.angle += rotator.speed;
    }
}


struct Rotator {
    axis: Vec3,
    angle: f32,
    speed: f32,
}