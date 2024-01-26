//! Module that defines both graphics primitives, and multiple graphics engines that make use of those primitives.
//! The graphics primitives are stored in the domain [`GraphicsState`].
//! The 3D graphics engine is [`G3D`]

mod state;
mod color;
mod shader;
mod scene;
mod buffer;
pub mod g3d;

use hecs::World;
pub use state::*;
pub use color::*;
pub use shader::*;
pub use scene::*;
pub use buffer::*;

use tracing::instrument;
use wgpu::{Color as WgpuColor, CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, SurfaceTexture};
use crate::math::Transform;
use crate::{RunContext, Game, AppBuilder, Stage, Plugin, Tracker, Camera};


/// Adds primitive [`GraphicsState`].
/// Adds a 2D and 3D graphics engine.
pub struct GraphicsPlugin;
impl Plugin for GraphicsPlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        builder.game()
            .init(|_| Scene::<g3d::Renderable>::new())
            .init(|game| {
                let state = game.get::<&GraphicsState>();
                let device = state.device.clone();
                let queue = state.queue.clone();
                g3d::G3D::new(device, queue)
            });
        builder.system(Stage::Render, render_3d);
    }
}

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
        match renderable.interpolation_mode {
            InterpolationMode::Interpolate => {
                render_cam.previous_projection = render_cam.projection;
                render_cam.projection = camera.projection;
            },
            InterpolationMode::Skip => {
                render_cam.previous_projection = camera.projection;
                render_cam.projection = camera.projection;
                renderable.interpolation_mode = InterpolationMode::Interpolate;
            },
            InterpolationMode::None => {
                render_cam.previous_projection = camera.projection;
                render_cam.projection = camera.projection;
            },
        }
    }
}

fn render_3d(game: &mut Game, ctx: RunContext) {

    let mut world = game.get::<&mut World>();
    let graphics_state = game.get::<&GraphicsState>();
    let mut g3d = game.get::<&mut g3d::G3D>();
    let mut g3d_scene = game.get::<&mut Scene<g3d::Renderable>>();

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
    enqueue_render(&graphics_state, &mut g3d_scene, &mut g3d, &surface_tex, &ctx);
    surface_tex.present();
}

#[instrument(skip_all)]
fn enqueue_render(
    graphics_state: &GraphicsState,
    g3d_scene: &mut Scene<g3d::Renderable>,
    g3d: &mut g3d::G3D,
    surface_tex: &SurfaceTexture,
    ctx: &RunContext,
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
        let flat_scene = g3d::flatten_scene(&g3d_scene, ctx.partial_ticks());
        let g3d_jobs = g3d.prepare_jobs(flat_scene, texture_format, depth_format);

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

        // Encodes 3D scene
        g3d.render_jobs(g3d_jobs, &mut pass);
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