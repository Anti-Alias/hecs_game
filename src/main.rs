use glam::Vec3;
use hecs_game::{App, EnginePlugin, AppBuilder, Color, Cuboid, Mesh, Handle, Material, Renderable, GraphicsState, GpuMesh, G3D, GpuMaterial};

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
    let (state, mut g3d) = builder
        .game()
        .all::<(&GraphicsState, &mut G3D)>();
    
    // Creates material and mesh.
    let material: Material = Color::BLUE.into();
    let mesh: Mesh = Cuboid {
        center: Vec3::new(0.0, 0.0, 0.0),
        half_extents: Vec3::new(0.5, 0.5, 0.5),
        color: Color::WHITE,
    }.into();

    // Uploads material and mesh to GPU.
    let gpu_material = GpuMaterial::from_material(&material, &state.device);
    let gpu_mesh = GpuMesh::from_mesh(&mesh, &state.device);

    // Places renderable in 3D scene.
    let renderable = Renderable::mat_mesh(Handle::new(gpu_material), Handle::new(gpu_mesh));
    g3d.scene_mut().insert_untracked(renderable);
}