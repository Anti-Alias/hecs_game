use std::collections::HashMap;
use std::mem::size_of;
use std::sync::Arc;
use glam::{Mat4, Affine3A};
use wgpu::{RenderPass, Device, Queue, RenderPipeline, BufferUsages, Buffer, BufferDescriptor, RenderPipelineDescriptor, PipelineLayoutDescriptor, VertexState, PrimitiveState, PrimitiveTopology, FrontFace, PolygonMode, FragmentState, TextureFormat, ColorTargetState, BlendState, ColorWrites, ShaderModuleDescriptor, ShaderSource};
use crate::{Handle, Slot, SceneGraph, HandleId, reserve_buffer, ShaderPreprocessor};
use crate::g3d::{GpuMaterial, GpuMesh, MeshVariant, MaterialVariant};
use derive_more::From;

const VERTEX_INDEX: u32 = 0;
const INSTANCE_INDEX: u32 = 1;
const MATERIAL_INDEX: u32 = 0;

/// A 3D graphics engine that stores its renderables in a scene graph.
pub struct G3D {
    pipelines: HashMap<PipelineKey, RenderPipeline>,    // Cache of render pipelines to use
    device: Arc<Device>,
    queue: Arc<Queue>,
    instance_buffer: Buffer,
}

impl G3D {

    /// New graphics engine with an empty scene graph.
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            pipelines: HashMap::new(),
            device: device.clone(),
            queue,
            instance_buffer: device.create_buffer(&BufferDescriptor {
                label: None,
                size: 0,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }

    /// Renders the entire scene graph
    pub fn render<'s: 'r, 'r>(
        &'s mut self,
        scene: &mut SceneGraph<Renderable>,
        pass: &mut RenderPass<'r>,
        texture_format: TextureFormat
    ) {
       
        // Prunes nodes that had their handles dropped
        scene.prune_nodes();

        // Propagate transforms of all renderables
        let init_transf = Mat4::IDENTITY;
        scene.propagate(init_transf, |parent_transf, renderable| {
            renderable.global_transform = parent_transf * renderable.transform;
            renderable.global_transform
        });

        // Collects renderables into instance batches (groups of renderables with the same material and mesh)
        let mut renderable_count = 0;
        let mut instance_batches: HashMap<InstanceKey, MatMeshInstances> = HashMap::new();
        for renderable in scene.iter() {

            // Extracts material and mesh from renderable. Skips if empty or not loaded.
            let RenderableKind::MatMesh(material_handle, mesh_handle) = &renderable.kind else { continue };
            let material_slot = material_handle.slot();
            let mesh_slot = mesh_handle.slot();
            let Some(material) = material_slot.loaded() else { continue };
            let Some(mesh) = mesh_slot.loaded() else { continue };

            // Creates pipeline compatible with material and mesh.
            // Does nothing if already cached.
            let pipeline_key = PipelineKey(mesh.variant, material.variant);
            self.pipelines
                .entry(pipeline_key)
                .or_insert_with(|| create_pipeline(&material, &mesh, texture_format, &self.device));

            // Fetches batch for material and mesh.
            // Creates it if it does not exist.
            let instance_batch = instance_batches
                .entry(InstanceKey {
                    material_id: material_handle.id(),
                    mesh_id: mesh_handle.id(),
                })
                .or_insert_with(|| MatMeshInstances {
                    material_slot,
                    mesh_slot,
                    pipeline_key,
                    instance_data: Vec::new(),
                });
            
            // Inserts instance data into that batch.
            instance_batch.instance_data.push(renderable.global_transform);
            renderable_count += 1;
        }

        // Reserve space for all instance data on the GPU buffer
        reserve_buffer(
            &mut self.instance_buffer,
            renderable_count * size_of::<Mat4>() as u64,
            &self.device
        );

        // Renders all instance batches
        let mut buffer_offset = 0;
        for instance_batch in instance_batches.into_values() {

            // Uploads instance data to section of instance buffer
            let transform_bytes: &[u8] = bytemuck::cast_slice(&instance_batch.instance_data);
            self.queue.write_buffer(&self.instance_buffer, buffer_offset, transform_bytes);

            // Gets material, mesh and pipeline for rendering.
            let material: &'s GpuMaterial = unsafe {
                std::mem::transmute(instance_batch.material_slot.loaded().unwrap())
            };
            let mesh: &'s GpuMesh = unsafe {
                std::mem::transmute(instance_batch.mesh_slot.loaded().unwrap())
            };
            let pipeline = self.pipelines.get(&instance_batch.pipeline_key).unwrap();

            // Draws instances of material / mesh
            let instance_range = buffer_offset .. buffer_offset+transform_bytes.len() as u64;
            let num_instances = instance_batch.instance_data.len() as u32;
            pass.set_pipeline(pipeline);
            pass.set_bind_group(MATERIAL_INDEX, &material.bind_group, &[]);                     // Material
            pass.set_vertex_buffer(VERTEX_INDEX, mesh.vertices.slice(..));                      // Mesh vertices
            pass.set_vertex_buffer(INSTANCE_INDEX, self.instance_buffer.slice(instance_range)); // Instance data
            pass.set_index_buffer(mesh.indices.slice(..), mesh.index_format);                   // Mesh indices
            pass.draw_indexed(0..mesh.num_indices, 0, 0..num_instances);
            buffer_offset += transform_bytes.len() as u64;
        }
    }
}

/**
 * Object that can be rendered in some way.
 */
pub struct Renderable {
    pub kind: RenderableKind,
    pub transform: Affine3A,
    global_transform: Mat4,
}

impl Renderable {
    
    pub fn new(kind: impl Into<RenderableKind>) -> Self {
        Self {
            kind: kind.into(),
            transform: Affine3A::IDENTITY,
            global_transform: Mat4::IDENTITY,
        }
    }

    pub fn empty() -> Self {
        Self::new(RenderableKind::Empty)
    }

    pub fn mat_mesh(material: Handle<GpuMaterial>, mesh: Handle<GpuMesh>) -> Self {
        Self::new(RenderableKind::MatMesh(material, mesh))
    }

    pub fn with_kind(mut self, kind: impl Into<RenderableKind>) -> Self {
        self.kind = kind.into();
        self
    }

    pub fn with_transform(mut self, transform: Affine3A) -> Self {
        self.transform = transform;
        self
    }
}

/// Different types of renderables.
#[derive(From)]
pub enum RenderableKind {
    /// A material and mesh combo.
    MatMesh(Handle<GpuMaterial>, Handle<GpuMesh>),
    /// No renderable content.
    /// Useful for grouping objects with no visible parent.
    Empty,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
struct PipelineKey(MeshVariant, MaterialVariant);

/// Key used to collect material/meshes into instances
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct InstanceKey {
    material_id: HandleId,
    mesh_id: HandleId,
}

/// Instance data for a material + mesh combo
struct MatMeshInstances<'a> {
    material_slot: Slot<'a, GpuMaterial>,
    mesh_slot: Slot<'a, GpuMesh>,
    pipeline_key: PipelineKey,
    instance_data: Vec<Mat4>,
}

/// Creates a pipeline compatible with the material and mesh supplied.
fn create_pipeline(
    material: &GpuMaterial,
    mesh: &GpuMesh,
    texture_format: TextureFormat,
    device: &Device
) -> RenderPipeline {

    // Extracts layout info and shader defs
    let mut shader_defs = ShaderPreprocessor::new();
    let material_layout = &material.layout;
    let mesh_layout = mesh.variant.layout(&mut shader_defs);
    let vertex_layout = mesh_layout.as_vertex_layout();

    // Generates shader module
    let shader_code = include_str!("shader.wgsl");
    let shader_code = shader_defs
        .preprocess(shader_code)
        .unwrap();
    println!("{shader_code}");
    let module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("g3d_module"),
        source: ShaderSource::Wgsl(shader_code.into()),
    });
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("g3d_layout"),
        bind_group_layouts: &[material_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("g3d_pipeline"),
        layout: Some(&layout),
        vertex: VertexState {
            module: &module,
            entry_point: "vertex_main",
            buffers: &[vertex_layout],
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(FragmentState {
            module: &module,
            entry_point: "fragment_main",
            targets: &[Some(ColorTargetState {
                format: texture_format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}