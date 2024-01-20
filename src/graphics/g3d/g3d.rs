use std::collections::HashMap;
use std::mem::size_of;
use std::sync::Arc;
use glam::{Mat4, Affine3A};
use wgpu::{RenderPass, Device, Queue, RenderPipeline, BufferUsages, Buffer, BufferDescriptor, RenderPipelineDescriptor, PipelineLayoutDescriptor, VertexState, PrimitiveState, PrimitiveTopology, FrontFace, PolygonMode, FragmentState, TextureFormat, ColorTargetState, BlendState, ColorWrites, ShaderModuleDescriptor, ShaderSource, VertexBufferLayout, VertexStepMode, VertexAttribute, VertexFormat, DepthStencilState, CompareFunction, StencilState, DepthBiasState};
use derive_more::From;
use crate::math::Transform;
use crate::{Handle, Slot, SceneGraph, HandleId, reserve_buffer, ShaderPreprocessor};
use crate::g3d::{GpuMaterial, GpuMesh, MeshVariant, MaterialVariant, Camera};

const INSTANCE_SLOT: u32 = 0;
const VERTEX_SLOT: u32 = 1;
const MATERIAL_INDEX: u32 = 0;

const INSTANCE_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
    array_stride: size_of::<Mat4>() as u64,
    step_mode: VertexStepMode::Instance,
    attributes: &[
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 0*4*4,
            shader_location: 0,
        },
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 1*4*4,
            shader_location: 1,
        },
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 2*4*4,
            shader_location: 2,
        },
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 3*4*4,
            shader_location: 3,
        },
    ],
};

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

    /// Generates render jobs for every camera in the scene graph.
    pub(crate) fn prepare_job<'s>(
        &mut self,
        scene: FlatScene<'s>,
        texture_format: TextureFormat,
        depth_format: TextureFormat,
    ) -> RenderJob<'s> {
        let mut renderable_count = 0;
        let mut instance_batches: HashMap<InstanceKey, MatMeshInstances> = HashMap::new();
        for mat_mesh in scene.mat_meshes {

            // Extracts MatMesh from renderable. Skips if empty or not loaded.
            let MatMesh(material_handle, mesh_handle) = mat_mesh.mat_mesh;
            let material_slot = material_handle.slot();
            let mesh_slot = mesh_handle.slot();
            let Some(material) = material_slot.loaded() else { continue };
            let Some(mesh) = mesh_slot.loaded() else { continue };

            // Creates pipeline compatible with material and mesh.
            // Does nothing if already cached.
            let pipeline_key = PipelineKey(mesh.variant, material.variant);
            self.pipelines
                .entry(pipeline_key)
                .or_insert_with(|| create_pipeline(&material, &mesh, texture_format, depth_format, &self.device));

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
            instance_batch.instance_data.push(mat_mesh.global_transform);
            renderable_count += 1;
        }
        RenderJob {
            instance_batches: instance_batches.into_values().collect(),
            renderable_count,
        }
    }

    /// Renders a job.
    pub fn render<'r>(&'r mut self, job: RenderJob<'r>, pass: &mut RenderPass<'r>) {

        // Reserve space for all instance data on the GPU buffer
        reserve_buffer(
            &mut self.instance_buffer,
            job.renderable_count * size_of::<Mat4>() as u64,
            &self.device
        );

        // Buffers draw calls for instance batches
        let mut buffer_offset = 0;
        let mut instance_bytes = Vec::new();
        for instance_batch in job.instance_batches {

            // Appends instance data to vec
            let transform_bytes: &[u8] = bytemuck::cast_slice(&instance_batch.instance_data);
            instance_bytes.extend_from_slice(transform_bytes);

            // Gets material, mesh and pipeline for rendering.
            let material: &'r GpuMaterial = unsafe {
                std::mem::transmute(instance_batch.material_slot.loaded().unwrap())
            };
            let mesh: &'r GpuMesh = unsafe {
                std::mem::transmute(instance_batch.mesh_slot.loaded().unwrap())
            };
            let pipeline = self.pipelines.get(&instance_batch.pipeline_key).unwrap();

            // Draws instances of material / mesh
            let instance_range = buffer_offset .. buffer_offset+transform_bytes.len() as u64;
            let num_instances = instance_batch.instance_data.len() as u32;
            pass.set_pipeline(pipeline);
            pass.set_bind_group(MATERIAL_INDEX, &material.bind_group, &[]);                     // Material
            pass.set_vertex_buffer(INSTANCE_SLOT, self.instance_buffer.slice(instance_range));  // Instance data
            pass.set_vertex_buffer(VERTEX_SLOT, mesh.vertices.slice(..));                       // Mesh vertices
            pass.set_index_buffer(mesh.indices.slice(..), mesh.index_format);                   // Mesh indices
            pass.draw_indexed(0..mesh.num_indices, 0, 0..num_instances);
            buffer_offset += transform_bytes.len() as u64;
        }
        self.queue.write_buffer(&self.instance_buffer, 0, &instance_bytes);
    }
}

/// Propagates transforms in the scene, and separates renderables into flat vecs separated by type.
pub(crate) fn flatten_scene<'a>(scene: &'a SceneGraph<Renderable>) -> FlatScene<'a> {
    let mut flat_scene = FlatScene::new();
    let init_transf = Mat4::IDENTITY;
    scene.propagate(init_transf, |parent_transf, renderable| {
        let local_transform = Affine3A::from(renderable.transform);
        let global_transform = parent_transf * local_transform;
        flat_scene.add(renderable, global_transform);
        global_transform
    });
    flat_scene
}

/// Collection of renderables to be rendered at a later time.
/// As long as a render job is alive, the required renderable resources are read-locked.
pub struct RenderJob<'a> {
    instance_batches: Vec<MatMeshInstances<'a>>,
    renderable_count: u64,
}

/**
 * Object that can be rendered in some way.
 */
pub struct Renderable {
    pub kind: RenderableKind,
    pub transform: Transform,
}

impl Renderable {
    
    pub fn new(kind: RenderableKind) -> Self {
        Self {
            kind,
            transform: Transform::IDENTITY,
        }
    }

    /**
     * Creates a [`MatMesh`] renderable.
     */
    pub fn mat_mesh(material: Handle<GpuMaterial>, mesh: Handle<GpuMesh>) -> Self {
        Self::new(RenderableKind::MatMesh(MatMesh(material, mesh)))
    }

    /**
     * Creates a [`Camera`] renderable.
     */
    pub fn camera(camera: Camera) -> Self {
        Self::new(RenderableKind::Camera(camera))
    }

    /**
     * Creates an empty renderable.
     */
    pub fn empty() -> Self {
        Self::new(RenderableKind::Empty)
    }
}

/// Different types of renderables.
#[derive(From)]
pub enum RenderableKind {
    /// A material and mesh combo.
    MatMesh(MatMesh),
    /// No renderable content.
    /// 3D perspective or orthographic camera.
    Camera(Camera),
    /// No renderable content.
    /// Useful for grouping objects with no visible parent.
    Empty,
}

/// Material mesh renderable.
pub struct MatMesh(Handle<GpuMaterial>, Handle<GpuMesh>);

/// MatMesh with its transform propagated.
pub struct FlatMatMesh<'a> {
    mat_mesh: &'a MatMesh,
    global_transform: Mat4,
}

/// Camera with its transform propagated.
pub struct FlatCamera<'a> {
    camera: &'a Camera,
    global_transform: Mat4,
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
    depth_format: TextureFormat,
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
    let module = device.create_shader_module(ShaderModuleDescriptor { label: Some("g3d_module"),
        source: ShaderSource::Wgsl(shader_code.into()),
    });
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("g3d_layout"),
        bind_group_layouts: &[material_layout],
        push_constant_ranges: &[],
    });

    // Creates pipeline
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("g3d_pipeline"),
        layout: Some(&layout),
        vertex: VertexState {
            module: &module,
            entry_point: "vertex_main",
            buffers: &[INSTANCE_LAYOUT, vertex_layout],
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
        depth_stencil: Some(DepthStencilState {
            format: depth_format,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }),
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

/// A flattened [`SceneGraph`] where renderable is separated by type.
pub(crate) struct FlatScene<'a> {
    mat_meshes: Vec<FlatMatMesh<'a>>,
    cameras: Vec<FlatCamera<'a>>,
}

impl<'a> FlatScene<'a> {

    fn new() -> Self {
        Self {
            mat_meshes: Vec::new(),
            cameras: Vec::new(),
        }
    }

    fn add(&mut self, renderable: &'a Renderable, global_transform: Mat4) {
        match &renderable.kind {
            RenderableKind::MatMesh(mat_mesh) => self.mat_meshes.push(FlatMatMesh {
                mat_mesh,
                global_transform,
            }),
            RenderableKind::Camera(camera) => self.cameras.push(FlatCamera {
                camera,
                global_transform,
            }),
            RenderableKind::Empty => {},
        }
    }
}