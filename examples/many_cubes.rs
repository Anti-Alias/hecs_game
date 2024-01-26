use std::f32::consts::TAU;
use glam::{Vec3, Quat};
use hecs_game::math::Transform;
use hecs_game::{g3d, App, AppBuilder, Camera, CameraController, Color, EnginePlugin, FlycamMode, FlycamPlugin, Game, GraphicsState, Handle, OrthographicProjector, PerspectiveProjector, RunContext, ScalingMode, Scene, Stage, StartEvent};
use hecs::World;
use rand::{SeedableRng, Rng};
use rand::rngs::SmallRng;

fn main() {
    let mut builder = App::builder();
    builder
        .plugin(EnginePlugin::default())
        .plugin(FlycamPlugin)
        .plugin(plugin);
    builder.run();
}

fn plugin(builder: &mut AppBuilder) {
    builder
        .event_handler(handle_start)
        .system(Stage::Update, rotate_cubes)
        .tick_rate(60.0);
}

fn handle_start(game: &mut Game, _event: &StartEvent, _ctx: &mut RunContext) {

    // Extracts domains
    let mut world       = game.get::<&mut World>();
    let mut scene       = game.get::<&mut Scene<g3d::Renderable>>();
    let state           = game.get::<&GraphicsState>();

    // Spawns flycam
    let cam_tracker = scene.insert(g3d::Renderable::camera());
    let cam_transform = Transform::default().with_xyz(0.0, 0.0, 1.0);
    world.spawn((
        cam_tracker,
        cam_transform,
        Camera::default(),
        CameraController {
            perspective: PerspectiveProjector {
                aspect_ratio: 1.0,
                near: 0.2,
                far: 1000.0,
                ..Default::default()
            },
            orthographic: OrthographicProjector {
                near: 0.2,
                far: 1000.0,
                ..Default::default()
            },
            t: 1.0,
            scaling_mode: ScalingMode::ScaleSmallest,
            flycam_mode: FlycamMode::Disabled,
            ..Default::default()
        },
    ));
    
    // Creates material
    let material = g3d::Material::from(Color::BLUE);
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
    for _ in 0..1000 {

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

struct Rotator {
    axis: Vec3,
    angle: f32,
    speed: f32,
}

fn rotate_cubes(game: &mut Game, ctx: RunContext) {
    let mut world = game.get::<&mut World>();
    let query = world.query_mut::<(&mut Transform, &mut Rotator)>();
    let delta = ctx.delta_secs();
    rayon::scope(|s| {
        for batch in query.into_iter_batched(1024) {
            s.spawn(|_| {
                for (_, (transform, rotator)) in batch {
                    transform.rotation = Quat::from_axis_angle(rotator.axis, rotator.angle);
                    rotator.angle += rotator.speed * delta;
                }
            });
        }
    });
}