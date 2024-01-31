use std::collections::HashMap;
use std::mem::size_of;
use std::sync::Arc;
use glam::{Mat4, Affine3A, Vec3};
use tracing::instrument;
use derive_more::From;
use wgpu::{BlendState, Buffer, BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Device, Face, FragmentState, FrontFace, PolygonMode, PrimitiveState, PrimitiveTopology, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, StencilState, TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode};
use crate::math::{lerp_matrices, Frustum, Sphere, Transform, Volume, AABB};
use crate::{reserve_buffer, AssetId, AssetState, AssetStorage, Handle, HasId, InterpolationMode, NodeId, Rect, Scene, ShaderPreprocessor, Texture, URect};
use crate::g3d::{Material, Mesh, MeshKey, Camera, CameraTarget};
use super::MaterialKey;

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
    instances: Buffer,
}

impl G3D {

    /// New graphics engine with an empty scene graph.
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            pipelines: HashMap::default(),
            device: device.clone(),
            queue,
            instances: device.create_buffer(&BufferDescriptor {
                label: None,
                size: 0,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }

    /// Generates render jobs for every camera in the scene graph.
    #[instrument(skip_all)]
    pub fn create_jobs<'s>(
        &mut self,
        flat_scene: FlatScene<'s>,
        texture_format: TextureFormat,
        depth_format: TextureFormat,
        materials: &'s AssetStorage<Material>,
        meshes: &'s AssetStorage<Mesh>,
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

            // Renders mat meshes.
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

                // Extracts material and mesh from renderable.
                // Skips if material or mesh have not done loading.
                // Skips if material has textures that are not done loading.
                let MatMesh(material_handle, mesh_handle) = flat_mat_mesh.mat_mesh;
                let AssetState::Loaded(material) = materials.get(material_handle) else { continue };
                let AssetState::Loaded(mesh) = meshes.get(mesh_handle) else { continue };
                let Some(prepared_material) = &material.prepared else { continue };
                
                // Creates pipeline compatible with material and mesh.
                // Does nothing if already cached.
                let pipeline_key = PipelineKey(mesh.key, prepared_material.key);
                let pipeline = self.pipelines
                    .entry(pipeline_key)
                    .or_insert_with(|| create_pipeline(
                        &material,
                        &mesh,
                        prepared_material.key.cull_mode,
                        texture_format,
                        depth_format,
                        &self.device
                    ));

                // Fetches instance batch for material and mesh.
                // Creates it if it does not exist.
                let instance_key = InstanceKey { material_id: material_handle.id(), mesh_id: mesh_handle.id() };
                let instance_batch = instance_batches
                    .entry(instance_key)
                    .or_insert_with(|| MatMeshInstances::new(material, mesh, pipeline_key));
                
                // Inserts instance data into that batch.
                instance_batch.instance_data.push(proj_view * flat_mat_mesh.global_transform);
                renderable_count += 1;
            }
            jobs.push(RenderJob {
                camera: flat_cam,
                instance_batches: instance_batches.into_values().collect(),
            });
        }
        RenderJobs { jobs, renderable_count }
    }

    /// Renders a collection of RenderJobs.
    #[instrument(skip_all)]
    pub fn submit_jobs<'r>(&'r mut self, jobs: RenderJobs<'r>, pass: &mut RenderPass<'r>) {

        // Reserves just enough room to store all instance data across all instance batches.
        reserve_buffer(
            &mut self.instances,
            jobs.renderable_count * size_of::<Mat4>() as u64,
            &self.device
        );

        for job in jobs.jobs {
            self.submit_job(job, pass);
        }
    }

    /// Renders a single RenderJob.
    fn submit_job<'r>(
        &'r self,
        job: RenderJob<'r>,
        pass: &mut RenderPass<'r>,
    ) {
        let mut buffer_offset = 0;
        let mut instance_bytes = Vec::new();

        if let Some(vp) = job.camera.viewport {
            let sc = URect::from(vp);
            pass.set_viewport(vp.origin.x, vp.origin.y, vp.size.x, vp.size.y, 0.0, 1.0);
            pass.set_scissor_rect(sc.origin.x, sc.origin.y, sc.size.x, sc.size.y);
        }

        for instance_batch in job.instance_batches {

            // Gets material, mesh and pipeline for rendering.
            let (material, mesh) = (instance_batch.mesh, instance_batch.mesh);

            // Collects instance bytes for this batch
            let transform_bytes: &[u8] = bytemuck::cast_slice(&instance_batch.instance_data);
            instance_bytes.extend_from_slice(transform_bytes);

            let pipeline = self.pipelines.get(&instance_batch.pipeline_key).unwrap();

            // Draws instances of a single material / mesh
            let instance_range = buffer_offset .. buffer_offset+transform_bytes.len() as u64;
            let num_instances = instance_batch.instance_data.len() as u32;
            pass.set_pipeline(pipeline);
            //pass.set_bind_group(MATERIAL_INDEX, &material.bind_group, &[]);                     // Material
            pass.set_vertex_buffer(INSTANCE_SLOT, self.instances.slice(instance_range));  // Instance data
            pass.set_vertex_buffer(VERTEX_SLOT, mesh.vertices.slice(..));                       // Mesh vertices
            pass.set_index_buffer(mesh.indices.slice(..), mesh.index_format);                   // Mesh indices
            pass.draw_indexed(0..mesh.num_indices, 0, 0..num_instances);
            buffer_offset += transform_bytes.len() as u64;
        }
        self.queue.write_buffer(&self.instances, 0, &instance_bytes);
    }
}

/// Creates a "flattened" version of the scene.
/// All renderables have their transforms propagated.
/// All renderables are put into separate flat vecs.
#[instrument(skip_all)]
pub(crate) fn flatten_scene<'a>(scene: &'a Scene<Renderable>, t: f32) -> FlatScene<'a> {
    let mut flat_scene = FlatScene::with_capacities(scene.len(), 1);
    let init_transf = Mat4::IDENTITY;
    scene.graph.propagate(init_transf, |parent_transf, renderable| {
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
                projection: lerp_matrices(camera.previous_projection, camera.projection, t),
                viewport: camera.viewport,
            }),
            RenderableKind::Empty => {},
        }
        global_transform
    });
    flat_scene
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
    camera: FlatCamera<'a>,
    instance_batches: Vec<MatMeshInstances<'a>>,
}

/**
 * Object that can be rendered in some way.
 */
pub struct Renderable {
    pub kind: RenderableKind,
    transform: Transform,
    previous_transform: Transform,
    pub volume: Option<Volume>,
    pub interpolation_mode: InterpolationMode,
}

impl Default for Renderable {
    fn default() -> Self {
        Self {
            kind: RenderableKind::Empty,
            transform: Transform::IDENTITY,
            previous_transform: Transform::IDENTITY,
            volume: None,
            interpolation_mode: InterpolationMode::Skip,
        }
    }
}

impl Renderable {

    /**
     * Creates an empty renderable.
     */
    pub fn empty() -> Self {
        Self::default()
    }

    /**
     * Creates a [`MatMesh`] renderable.
     */
    pub fn mat_mesh(material: Handle<Material>, mesh: Handle<Mesh>) -> Self {
        Self {
            kind: RenderableKind::MatMesh(MatMesh(material, mesh)),
            ..Default::default()
        }
    }

    /**
     * Creates a [`Camera`] renderable.
     */
    pub fn camera() -> Self {
        Self {
            kind: RenderableKind::Camera(Camera::default()),
            ..Default::default()
        }
    }

    pub fn with_kind(mut self, kind: RenderableKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_mat_mesh(mut self, material: Handle<Material>, mesh: Handle<Mesh>) -> Self {
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

    pub fn with_interpolation_mode(mut self, interpolation_mode: InterpolationMode) -> Self {
        self.interpolation_mode = interpolation_mode;
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

    pub fn transform(&self) -> Transform {
        self.transform
    }

    pub fn set_transform(&mut self, transform: Transform) {
        match self.interpolation_mode {
            InterpolationMode::Interpolate => {
                self.previous_transform = self.transform;
                self.transform = transform;
            },
            InterpolationMode::Skip => {
                self.transform = transform;
                self.previous_transform = transform;
                self.interpolation_mode = InterpolationMode::Interpolate;
            },
            InterpolationMode::None => {
                self.transform = transform;
            },
        }
    }
}

impl HasId for Renderable {
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

impl RenderableKind {
    pub fn as_mat_mesh(&self) -> Option<&MatMesh> {
        match self {
            RenderableKind::MatMesh(mat_mesh) => Some(mat_mesh),
            _ => None,
        }
    }

    pub fn as_mat_mesh_mut(&mut self) -> Option<&mut MatMesh> {
        match self {
            RenderableKind::MatMesh(mat_mesh) => Some(mat_mesh),
            _ => None,
        }
    }

    pub fn as_camera(&self) -> Option<&Camera> {
        match self {
            RenderableKind::Camera(camera) => Some(camera),
            _ => None,
        }
    }

    pub fn as_camera_mut(&mut self) -> Option<&mut Camera> {
        match self {
            RenderableKind::Camera(camera) => Some(camera),
            _ => None,
        }
    }
}

/// Material mesh renderable.
pub struct MatMesh(Handle<Material>, Handle<Mesh>);

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
    viewport: Option<Rect>,
}

/// Used to select a pipeline from a cache.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
struct PipelineKey(MeshKey, MaterialKey);
impl identity_hash::IdentityHashable for PipelineKey {}

/// Key used to collect material/meshes into instances
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct InstanceKey {
    material_id: AssetId,
    mesh_id: AssetId,
}

/// Instance data for a material + mesh combo
struct MatMeshInstances<'a> {
    material: &'a Material,
    mesh: &'a Mesh,
    pipeline_key: PipelineKey,
    instance_data: Vec<Mat4>,
}

impl<'a> MatMeshInstances<'a> {
    pub fn new(
        material: &'a Material,
        mesh: &'a Mesh,
        pipeline_key: PipelineKey,
    ) -> Self {
        Self {
            material,
            mesh,
            pipeline_key,
            instance_data: Vec::new(),
        }
    }
}

/// Creates a pipeline compatible with the material and mesh supplied.
fn create_pipeline(
    material: &Material,
    mesh: &Mesh,
    cull_mode: Option<Face>,
    texture_format: TextureFormat,
    depth_format: TextureFormat,
    device: &Device
) -> RenderPipeline {

    // Extracts layout info and shader defs
    let mut shader_defs = ShaderPreprocessor::new();
    let mesh_layout = mesh.key.layout(&mut shader_defs);
    let vertex_layout = mesh_layout.as_vertex_layout();

    // Generates shader module
    let shader_code = include_str!("shader.wgsl");
    let shader_code = shader_defs
        .preprocess(shader_code)
        .unwrap();
    let module = device.create_shader_module(ShaderModuleDescriptor { label: Some("g3d_module"),
        source: ShaderSource::Wgsl(shader_code.into()),
    });
    // let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
    //     label: Some("g3d_layout"),
    //     bind_group_layouts: &[material_layout],
    //     push_constant_ranges: &[],
    // });

    // Creates pipeline
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("g3d_pipeline"),
        layout: None,
        //layout: Some(&layout),
        vertex: VertexState {
            module: &module,
            entry_point: "vertex_main",
            buffers: &[INSTANCE_LAYOUT, vertex_layout],
        },
        fragment: Some(FragmentState {
            module: &module,
            entry_point: "fragment_main",
            targets: &[Some(ColorTargetState {
                format: texture_format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode,
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