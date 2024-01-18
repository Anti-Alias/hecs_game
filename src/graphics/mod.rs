//! Module that defines both graphics primitives, and multiple graphics engines that make use of those primitives.
//! The graphics primitives are stored in the domain [`GraphicsState`].
//! The 3D graphics engine is [`G3D`]

mod state;
mod g3d;
mod color;
mod shader;
mod scene;
mod buffer;

pub use state::*;
pub use g3d::*;
pub use color::*;
pub use shader::*;
pub use scene::*;
pub use buffer::*;

use wgpu::{CommandEncoderDescriptor, RenderPassDescriptor, RenderPassColorAttachment, Operations, LoadOp, Color as WgpuColor, StoreOp};
use crate::{RunContext, Game, AppBuilder, Stage, Plugin};


/// Adds primitive [`GraphicsState`].
/// Adds a 2D and 3D graphics engine.
pub struct GraphicsPlugin;
impl Plugin for GraphicsPlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        builder
            .game()
            .init(|game| {
                let state = game.get::<&GraphicsState>();
                let device = state.device.clone();
                let queue = state.queue.clone();
                G3D::new(device, queue)
            });
        builder.add_system(Stage::Render, render, true);        
    }
}

fn render(game: &mut Game, _ctx: RunContext) {
    
    // Extracts resources for rendering
    let (graphics_state, mut g3d) = game.all::<(&GraphicsState, &mut G3D)>();
    let surface_tex = match graphics_state.surface().get_current_texture() {
        Ok(surface_tex) => surface_tex,
        Err(err) => {
            log::error!("{err}");
            return;
        }
    };

    // Encodes rendering commands
    let view = surface_tex.texture.create_view(&Default::default());
    let mut encoder = graphics_state.device.create_command_encoder(&CommandEncoderDescriptor::default());
    {
        // Clear screen
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
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Render 3D graphics
        g3d.render(&mut pass, graphics_state.surface_config().format);
    }
    
    // Submits encoded commands
    let commands = [encoder.finish()];
    graphics_state.queue.submit(commands);
    surface_tex.present();
}