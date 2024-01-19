use glam::Vec3;
use hecs::World;
use hecs_game::{g3d, App, EnginePlugin, AppBuilder, Color, Handle, GraphicsState, SceneGraph};

fn main() {
    env_logger::init();
    let mut builder = App::builder();
    builder
        .plugin(EnginePlugin)
        .plugin(plugin);
    builder.run();
}

fn plugin(builder: &mut AppBuilder) {

    // Extracts domains
    let (mut world, state, mut scene) = builder
        .game()
        .all::<(&mut World, &GraphicsState, &mut SceneGraph<g3d::Renderable>)>();
    
    // Creates material and mesh.
    let material: g3d::Material = Color::BLUE.into();
    let mesh: g3d::Mesh = g3d::Mesh::from(g3d::Cuboid {
        center: Vec3::new(0.0, 0.0, 0.0),
        half_extents: Vec3::new(0.5, 0.5, 0.5),
        color: Color::BLUE,
    });

    // Uploads material and mesh to GPU.
    let gpu_material = g3d::GpuMaterial::from_material(&material, &state.device);
    let gpu_mesh = g3d::GpuMesh::from_mesh(&mesh, &state.device);

    // Places renderable in 3D scene.
    let tracker = scene.insert(g3d::Renderable::mat_mesh(
        Handle::new(gpu_material),
        Handle::new(gpu_mesh)
    ));

    // Spawns entity
    world.spawn((tracker,));
}