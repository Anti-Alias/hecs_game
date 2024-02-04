use std::f32::consts::TAU;
use glam::{Vec3, Quat};
use hecs_game::math::Transform;
use hecs_game::{g3d, App, AssetManager, Camera, CameraController, Color, EnginePlugin, FlycamMode, FlycamPlugin, Game, GraphicsState, OrthographicProjector, PerspectiveProjector, RunContext, ScalingMode, Scene, Stage, StartEvent};
use hecs::World;
use rand::{SeedableRng, Rng};
use rand::rngs::SmallRng;
use wgpu::Face;

fn main() {
    let mut builder = App::builder();
    builder
        .plugin(EnginePlugin::default())
        .plugin(FlycamPlugin)
        .system(Stage::Update, rotate_cubes)
        .tick_rate(60.0)
        .event_handler(handle_start);
    builder.run();
}

fn handle_start(game: &mut Game, _event: &StartEvent, _ctx: &mut RunContext) {

    // Extracts domains
    let mut world       = game.get::<&mut World>();
    let mut scene       = game.get::<&mut Scene<g3d::Renderable>>();
    let state           = game.get::<&GraphicsState>();
    let mut assets      = game.get::<&mut AssetManager>();

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

    let mut rng = SmallRng::seed_from_u64(100);

    // Creates material
    let texture = assets.load("cube_texture.png");
    let material = assets.insert(g3d::Material {
        base_color_texture: Some(texture),
        cull_mode: Some(Face::Back),
        ..Default::default()
    });
    
    // Creates mesh
    let mesh: g3d::MeshData = g3d::MeshData::from(g3d::Cuboid {
        center: Vec3::new(0.0, 0.0, 0.0),
        half_extents: Vec3::new(0.5, 0.5, 0.5),
        color: Color::WHITE,
    });
    let mesh = g3d::Mesh::from_data(&mesh, &state.device);
    let mesh = assets.insert(mesh);

    // Creates colored mesh
    let mut colored_mesh: g3d::MeshData = g3d::MeshData::from(g3d::Cuboid {
        center: Vec3::new(0.0, 0.0, 0.0),
        half_extents: Vec3::new(0.5, 0.5, 0.5),
        color: Color::WHITE,
    });
    colored_mesh.colors = Some(rand_vertex_colors(&mut rng));
    let colored_mesh = g3d::Mesh::from_data(&colored_mesh, &state.device);
    let colored_mesh = assets.insert(colored_mesh);
    
    // Spawns cubes
    for _ in 0..100_000 {

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
            true => mesh.clone(),
            false => colored_mesh.clone(),
        };

        // Spawns cube with above data
        let renderable = g3d::Renderable::empty()
            .with_mat_mesh(material.clone(), mesh)
            .with_aabb_volume(Vec3::ZERO, Vec3::splat(0.5));
        let renderable = scene.insert(renderable);
        world.spawn((renderable, transform, rotator));
    }
}

fn rand_vertex_colors(rng: &mut SmallRng) -> Vec<Color> {
    let mut vertices = Vec::with_capacity(24);
    for _ in 0..24 {
        vertices.push(Color {
            r: rng.gen::<f32>() * 2.0 - 1.0,
            g: rng.gen::<f32>() * 2.0 - 1.0,
            b: rng.gen::<f32>() * 2.0 - 1.0,
            a: 1.0,
        })
    }
    vertices
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