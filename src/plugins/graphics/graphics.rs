use hecs::World;
use tracing::instrument;
use wgpu::{Color as WgpuColor, CommandEncoderDescriptor, Device, LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, SurfaceTexture};
use crate::g3d::{Material, Mesh, Renderable};
use crate::math::Transform;
use crate::{g3d, AppBuilder, AssetManager, AssetStorage, Camera, Game, GraphicsState, Plugin, RunContext, Scene, SceneGraph, Stage, Texture, TextureLoader, Tracker};


/// Adds primitive [`GraphicsState`].
/// Adds a 2D and 3D graphics engine.
pub struct GraphicsPlugin;
impl Plugin for GraphicsPlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        builder.system(Stage::Render, render_3d);
        let game = builder.game();
        game.add(Scene::<g3d::Renderable>::new());
        let (device, queue) = {
            let state = game.get::<&GraphicsState>();
            (state.device.clone(), state.queue.clone())
        };
        game.add(g3d::G3D::new(device.clone(), queue.clone()));
        let mut assets = game.get::<&mut AssetManager>();
        assets.add_loader(TextureLoader { device, queue, }).unwrap();
    }
}

#[instrument(skip_all)]
fn sync_graphics(world: &mut World, g3d_scene: &mut SceneGraph<g3d::Renderable>) {
    
    // Syncs transforms
    let renderable_query = world.query_mut::<(&Transform, &Tracker<g3d::Renderable>)>();
    rayon::scope(|s| {
        for batch in renderable_query.into_iter_batched(10000) {
            s.spawn(|_| {
                for (_, (transform, tracker)) in batch {
                    let renderable = unsafe {
                        g3d_scene.get_mut_unsafe(tracker.id())
                    };
                    let Some(renderable) = renderable else { continue };
                    renderable.set_transform(*transform);
                }
            });
        }
    });

    // Syncs cameras
    let camera_query = world.query_mut::<(&Camera, &Tracker<g3d::Renderable>)>();
    for (_, (camera, tracker)) in camera_query {
        let Some(renderable) = g3d_scene.get_mut(tracker.id()) else { continue };
        let Some(render_cam) = renderable.kind.as_camera_mut() else { continue };
        render_cam.viewport = camera.viewport;
        render_cam.set_projection(camera.projection);
    }
}

fn render_3d(game: &mut Game, ctx: RunContext) {

    let mut world           = game.get::<&mut World>();
    let graphics_state      = game.get::<&GraphicsState>();
    let mut g3d             = game.get::<&mut g3d::G3D>();
    let mut g3d_scene       = game.get::<&mut Scene<g3d::Renderable>>();
    let assets              = game.get::<&AssetManager>();

    if ctx.is_tick() {
        let g3d_scene = &mut g3d_scene.graph;
        sync_graphics(&mut world, g3d_scene);
    }
    
    let surface_tex = match graphics_state.surface().get_current_texture() {
        Ok(surface_tex) => surface_tex,
        Err(err) => {
            log::error!("{err}");
            return;
        }
    };

    let textures = assets.storage::<Texture>().unwrap();
    let meshes = assets.storage::<Mesh>().unwrap();
    let mut materials = assets.storage::<Material>().unwrap();

    prepare_materials(&mut materials, &textures, &graphics_state.device);
    enqueue_render(&graphics_state, &mut g3d_scene, &mut g3d, &surface_tex, ctx.partial_ticks(), &materials, &meshes);
    surface_tex.present();
}

fn prepare_materials(
    materials: &mut AssetStorage<Material>,
    textures: &AssetStorage<Texture>,
    device: &Device,
) {
    for material in materials.values_mut() {
        let Some(material) = material.as_loaded_mut() else { continue };
        material.prepare(textures, device);
    }
}

#[instrument(skip_all)]
fn enqueue_render(
    graphics_state: &GraphicsState,
    g3d_scene: &mut Scene<g3d::Renderable>,
    g3d: &mut g3d::G3D,
    surface_tex: &SurfaceTexture,
    partial_ticks: f32,
    materials: &AssetStorage<Material>,
    meshes: &AssetStorage<Mesh>,
) {
    let texture_format = graphics_state.format();
    let depth_format = graphics_state.depth_format();
    let depth_view = graphics_state.depth_view();

    // Removes nodes that are no longer tracked
    g3d_scene.prune_nodes();

    // Traverses scene and encodes commands
    let view = surface_tex.texture.create_view(&Default::default());
    let mut encoder = graphics_state.device.create_command_encoder(&CommandEncoderDescriptor::default());
    {
        // Flattens scene, and creates render jobs
        let flat_scene = g3d::flatten_scene(&g3d_scene, partial_ticks);
        let g3d_jobs = g3d.create_jobs(flat_scene, texture_format, depth_format, &materials, &meshes);

        // Creates render pass
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(WgpuColor::GREEN),
                        store: StoreOp::Store,
                    },
                })
            ],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Submits render jobs
        g3d.submit_jobs(g3d_jobs, &mut pass);
    }

    // Submits render commands
    let commands = [encoder.finish()];
    graphics_state.queue.submit(commands);
}

/// Determines how
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub enum InterpolationMode {
    /// Graphics will interpolate between previous and current state.
    /// Small visual latency.
    /// Good for high refresh-rate monitors.
    Interpolate,
    /// Graphics will not interpolate this tick.
    /// Moves to Interpolate state after.
    #[default]
    Skip,
    /// Graphics will be shown at current location only.
    /// Good for consistency, but looks choppy if frame rate is higher than tick rate.
    None,
}