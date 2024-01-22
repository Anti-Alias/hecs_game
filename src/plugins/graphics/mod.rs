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
use wgpu::{CommandEncoderDescriptor, RenderPassDescriptor, RenderPassColorAttachment, Operations, LoadOp, Color as WgpuColor, StoreOp, RenderPassDepthStencilAttachment};
use crate::math::Transform;
use crate::{RunContext, Game, AppBuilder, Stage, Plugin, Tracker, Projection};
use crate::g3d::RenderableKind;


/// Adds primitive [`GraphicsState`].
/// Adds a 2D and 3D graphics engine.
pub struct GraphicsPlugin;
impl Plugin for GraphicsPlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        builder.game()
            .init(|_| SceneGraph::<g3d::Renderable>::new())
            .init(|game| {
                let state = game.get::<&GraphicsState>();
                let device = state.device.clone();
                let queue = state.queue.clone();
                g3d::G3D::new(device, queue)
            });
        builder
            .add_system(Stage::RenderSyncPreUpdate, sync_previous_state)
            .add_system(Stage::RenderSyncPostUpdate, sync_current_state)
            .add_system(Stage::Render, render_3d);
    }
}

fn sync_previous_state(game: &mut Game, _ctx: RunContext) {

    let (mut g3d_scene, mut world) = game.all::<(
        &mut SceneGraph<g3d::Renderable>,
        &mut World,
    )>();

    for (_, (transform, tracker)) in world.query_mut::<(&Transform, &Tracker<g3d::Renderable>)>() {
        let Some(renderable) = g3d_scene.get_mut(tracker.id()) else { continue };
        renderable.previous_transform = *transform;
    }

    for (_, (projection, tracker)) in world.query_mut::<(&Projection, &Tracker<g3d::Renderable>)>() {
        let Some(renderable) = g3d_scene.get_mut(tracker.id()) else { continue };
        let RenderableKind::Camera(camera) = &mut renderable.kind else { continue };
        camera.previous_projection = projection.0;
    }
}

fn sync_current_state(game: &mut Game, _ctx: RunContext) {

    let (mut g3d_scene, mut world) = game.all::<(
        &mut SceneGraph<g3d::Renderable>,
        &mut World,
    )>();

    for (_, (transform, tracker)) in world.query_mut::<(&Transform, &Tracker<g3d::Renderable>)>() {
        let Some(renderable) = g3d_scene.get_mut(tracker.id()) else { continue };
        renderable.transform = *transform;
    }
    for (_, (projection, tracker)) in world.query_mut::<(&Projection, &Tracker<g3d::Renderable>)>() {
        let Some(renderable) = g3d_scene.get_mut(tracker.id()) else { continue };
        let RenderableKind::Camera(camera) = &mut renderable.kind else { continue };
        camera.projection = projection.0;
    }
}

fn render_3d(game: &mut Game, ctx: RunContext) {

    // Extracts resources for rendering
    let (graphics_state, mut g3d_scene, mut g3d) = game.all::<(
        &GraphicsState,
        &mut SceneGraph<g3d::Renderable>,
        &mut g3d::G3D,
    )>();
    let surface_tex = match graphics_state.surface().get_current_texture() {
        Ok(surface_tex) => surface_tex,
        Err(err) => {
            log::error!("{err}");
            return;
        }
    };
    let texture_format = graphics_state.surface_format();
    let depth_format = graphics_state.depth_format();
    let depth_view = graphics_state.depth_view();

    // Removes nodes that are no longer tracked
    g3d_scene.prune_nodes();

    // Renders scene
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

        // Submits rendering jobs to graphics engines
        g3d.render_jobs(g3d_jobs, &mut pass);
    }
    
    // Encoded render
    let commands = [encoder.finish()];
    graphics_state.queue.submit(commands);
    surface_tex.present();
}