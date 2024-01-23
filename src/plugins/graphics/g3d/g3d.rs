use std::mem::size_of;
use std::sync::Arc;
use glam::{Mat4, Affine3A, Vec3};
use tracing::instrument;
use wgpu::{RenderPass, Device, Queue, RenderPipeline, BufferUsages, Buffer, BufferDescriptor, RenderPipelineDescriptor, PipelineLayoutDescriptor, VertexState, PrimitiveState, PrimitiveTopology, FrontFace, PolygonMode, FragmentState, TextureFormat, ColorTargetState, BlendState, ColorWrites, ShaderModuleDescriptor, ShaderSource, VertexBufferLayout, VertexStepMode, VertexAttribute, VertexFormat, DepthStencilState, CompareFunction, StencilState, DepthBiasState};
use derive_more::From;
use crate::{Handle, Slot, SceneGraph, HandleId, reserve_buffer, ShaderPreprocessor, Trackee, NodeId, HashMap};
use crate::math::{Transform, Frustum, Volume, AABB, Sphere};
use crate::g3d::{GpuMaterial, GpuMesh, MeshVariant, MaterialVariant, Camera, CameraTarget};

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
pub(crate) struct G3D {
    pipelines: HashMap<PipelineKey, RenderPipeline>,    // Cache of render pipelines to use
    device: Arc<Device>,
    queue: Arc<Queue>,
    instance_buffer: Buffer,
}

impl G3D {

    /// New graphics engine with an empty scene graph.
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            pipelines: HashMap::default(),
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
    #[instrument(skip_all)]
    pub fn prepare_jobs<'s>(
        &mut self,
        flat_scene: FlatScene<'s>,
        texture_format: TextureFormat,
        depth_format: TextureFormat,
    ) -> RenderJobs<'s> {
        
        let mut jobs = Vec::new();
        let mut renderable_count = 0;

        // Collects N RenderJobs for N cameras.
        for flat_cam in flat_scene.flat_cams {
            let mut instance_batches: HashMap<InstanceKey, MatMeshInstances> = HashMap::default();
            let proj = flat_cam.projection;
            let view = flat_cam.global_transform.inverse();
            let proj_view = proj * view;
            let frustum = Frustum::from(proj_view);
            for flat_mat_mesh in &flat_scene.flat_mat_meshes {

                // Skips mat mesh if it has a bounding volume and it not in the frustum.
                match flat_mat_mesh.volume {
                    Some(Volume::Sphere(sphere)) => {
                        let global_sphere = sphere.transform(flat_mat_mesh.global_transform);
                        if !frustum.contains_sphere(global_sphere) {
                            continue;
                        }
                    },
                    Some(Volume::AABB(aabb)) => {
                        let global_aabb = aabb.transform(flat_mat_mesh.global_transform);
                        if !frustum.contains_aabb(global_aabb) {
                            continue;
                        }
                    },
                    None => {}
                }

                // Extracts material and mesh from renderable. Skips if not loaded.
                let MatMesh(material_handle, mesh_handle) = flat_mat_mesh.mat_mesh;
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

                // Fetches instance batch for material and mesh.
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
                instance_batch.instance_data.push(proj_view * flat_mat_mesh.global_transform);
                renderable_count += 1;
            }
            jobs.push(RenderJob {
                instance_batches: instance_batches.into_values().collect(),
            });
        }
        RenderJobs
         { jobs, renderable_count }
    }

    /// Renders a collection of RenderJobs.
    #[instrument(skip_all)]
    pub fn render_jobs<'s: 'r, 'r>(&'s mut self, jobs: RenderJobs<'r>, pass: &mut RenderPass<'r>) {

        // Reserves just enough room to store all instance data across all instance batches.
        reserve_buffer(
            &mut self.instance_buffer,
            jobs.renderable_count * size_of::<Mat4>() as u64,
            &self.device
        );

        for job in jobs.jobs {
            self.render_job(job, pass);
        }
    }

    /// Renders a single RenderJob.
    fn render_job<'s: 'r, 'r>(&'s self, job: RenderJob<'r>, pass: &mut RenderPass<'r>) {
        let mut buffer_offset = 0;
        let mut instance_bytes = Vec::new();
        for instance_batch in job.instance_batches {

            // Collects instance bytes for this batch
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

            // Draws instances of a single material / mesh
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

/// Creates a "flattened" version of the scene.
/// All renderables have their transforms propagated.
/// All renderables are put into separate flat vecs.
#[instrument(skip_all)]
pub(crate) fn flatten_scene<'a>(scene: &'a SceneGraph<Renderable>, t: f32) -> FlatScene<'a> {
    let mut flat_scene = FlatScene::with_capacities(scene.len(), 1);
    let init_transf = Mat4::IDENTITY;
    scene.propagate(init_transf, |parent_transf, renderable| {
        let local_transform = renderable.previous_transform.lerp(renderable.transform, t);
        let local_affine = Affine3A::from(local_transform);
        let global_transform = parent_transf * local_affine;
        match &renderable.kind {
            RenderableKind::MatMesh(mat_mesh) => flat_scene.flat_mat_meshes.push(FlatMatMesh {
                mat_mesh,
                global_transform,
                volume: renderable.volume,
            }),
            RenderableKind::Camera(camera) => flat_scene.flat_cams.push(FlatCamera {
                global_transform,
                _target: &camera.target,
                projection: lerp_mats(camera.previous_projection, camera.projection, t),
            }),
            RenderableKind::Empty => {},
        }
        global_transform
    });
    flat_scene
}

fn lerp_mats(a: Mat4, b: Mat4, t: f32) -> Mat4 {
    let col0 = a.col(0).lerp(b.col(0), t);
    let col1 = a.col(1).lerp(b.col(1), t);
    let col2 = a.col(2).lerp(b.col(2), t);
    let col3 = a.col(3).lerp(b.col(3), t);
    Mat4::from_cols(col0, col1, col2, col3)
}

// Collection of render jobs to render later.
pub struct RenderJobs<'a> {
    jobs: Vec<RenderJob<'a>>,
    renderable_count: u64,
}

/// Collection of "flattened" renderables to be rendered at a later time.
/// Note: As long as a render job is alive, the required renderable resources are read-locked.
/// This is necessary in order for the render pass to have stable pointers for its lifetime.
/// A RenderJob must outlive the render pass that uses it.
struct RenderJob<'a> {
    instance_batches: Vec<MatMeshInstances<'a>>,
}

/**
 * Object that can be rendered in some way.
 */
pub struct Renderable {
    pub kind: RenderableKind,
    pub transform: Transform,
    pub previous_transform: Transform,
    pub volume: Option<Volume>,
}

impl Renderable {

    /**
     * Creates an empty renderable.
     */
    pub fn empty() -> Self {
        Self {
            kind: RenderableKind::Empty,
            transform: Transform::IDENTITY,
            previous_transform: Transform::IDENTITY,
            volume: None,
        }
    }

    /**
     * Creates a [`MatMesh`] renderable.
     */
    pub fn mat_mesh(material: Handle<GpuMaterial>, mesh: Handle<GpuMesh>) -> Self {
        Self {
            kind: RenderableKind::MatMesh(MatMesh(material, mesh)),
            transform: Transform::IDENTITY,
            previous_transform: Transform::IDENTITY,
            volume: None,
        }
    }

    /**
     * Creates a [`Camera`] renderable.
     */
    pub fn camera() -> Self {
        Self {
            kind: RenderableKind::Camera(Camera::default()),
            transform: Transform::IDENTITY,
            previous_transform: Transform::IDENTITY,
            volume: None,
        }
    }

    pub fn with_kind(mut self, kind: RenderableKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_mat_mesh(mut self, material: Handle<GpuMaterial>, mesh: Handle<GpuMesh>) -> Self {
        self.kind = RenderableKind::MatMesh(MatMesh(material, mesh));
        self
    }

    pub fn with_camera(mut self) -> Self {
        self.kind = RenderableKind::Camera(Camera::default());
        self
    }

    pub fn with_empty(mut self) -> Self {
        self.kind = RenderableKind::Empty;
        self
    }

    pub fn with_volume(mut self, volume: Volume) -> Self {
        self.volume = Some(volume);
        self
    }

    pub fn with_aabb_volume(mut self, center: Vec3, extents: Vec3) -> Self {
        self.volume = Some(Volume::AABB(AABB::new(center, extents)));
        self
    }

    pub fn with_sphere_volume(mut self, center: Vec3, radius: f32) -> Self {
        self.volume = Some(Volume::Sphere(Sphere::new(center, radius)));
        self
    }
}

impl Trackee for Renderable {
    type Id = NodeId;
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
    volume: Option<Volume>,
}

/// Camera with its transform propagated.
pub struct FlatCamera<'a> {
    _target: &'a CameraTarget,
    projection: Mat4,
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
    flat_mat_meshes: Vec<FlatMatMesh<'a>>,
    flat_cams: Vec<FlatCamera<'a>>,
}

impl<'a> FlatScene<'a> {

    pub fn with_capacities(mat_meshes: usize, cams: usize) -> Self {
        Self {
            flat_mat_meshes: Vec::with_capacity(mat_meshes),
            flat_cams: Vec::with_capacity(cams),
        }
    }
}