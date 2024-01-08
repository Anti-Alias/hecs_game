use glam::{Mat4, Affine3A};
use wgpu::RenderPass;
use crate::{GraphicsState, Material, Mesh, Handle, SceneGraph};

pub struct G3D {
    pub scene: SceneGraph<MatMesh>,
}

impl G3D {

    pub fn new() -> Self {
        Self {
            scene: SceneGraph::new()
        }
    }

    pub fn render(&mut self, _pass: &mut RenderPass, _graphics_state: &GraphicsState) {
        self.propagate_transforms();
    }

    fn propagate_transforms(&mut self) {
        let init_transf = Mat4::IDENTITY;
        self.scene.propagate(init_transf, |parent_transf, node| {
            node.value.global_transform = parent_transf * node.value.transform;
            node.value.global_transform
        })
    }
}


/**
 * Renderable.
 * A [`Material`](crate::Material) associated with a [`Mesh`](crate::Mesh).
 */
#[derive(Clone)]
pub struct MatMesh {
    pub material: Handle<Material>,
    pub mesh: Handle<Mesh>,
    pub transform: Affine3A,
    global_transform: Mat4,
}

impl MatMesh {
    pub fn new(material: Handle<Material>, mesh: Handle<Mesh>) -> Self {
        Self {
            material,
            mesh,
            transform: Affine3A::IDENTITY,
            global_transform: Mat4::IDENTITY,
        }
    }
}
