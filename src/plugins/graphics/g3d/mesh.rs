use std::mem::size_of;
use bytemuck::bytes_of;
use wgpu::util::{DeviceExt, BufferInitDescriptor};
use wgpu::{VertexBufferLayout, VertexStepMode, VertexAttribute, VertexFormat, Buffer, Device, BufferUsages, IndexFormat};
use glam::{Vec3, Vec2};
use bitflags::bitflags;
use crate::{Asset, Color, ShaderPreprocessor};

/**
 * A 3D mesh.
*/
#[derive(Clone, Default)]
pub struct MeshData {
    pub indices:    Vec<u32>,
    pub positions:  Vec<Vec3>,
    pub colors:     Option<Vec<Color>>,
    pub normals:    Option<Vec<Vec3>>,
    pub uvs:        Option<Vec<Vec2>>,
}
impl MeshData {
    const POSITION_LOCATION: u32    = 4;
    const COLOR_LOCATION: u32       = 5;
    const NORMAL_LOCATION: u32      = 6;
    const UV_LOCATION: u32          = 7;

    const POSITION_SIZE: usize      = size_of::<Vec3>();
    const COLOR_SIZE: usize         = size_of::<Color>();
    const NORMAL_SIZE: usize        = size_of::<Vec3>();
    const UV_SIZE: usize            = size_of::<Vec2>();

    pub fn new() -> Self {
        Self {
            indices: Vec::new(),
            positions: Vec::new(),
            colors: None,
            uvs: None,
            normals: None,
        }
    }

    /**
     * Computes the [`MeshVariant`].
     */
    pub fn key(&self) -> MeshKey {
        let mut variant = MeshKey::NONE;
        if self.colors.is_some() {
            variant |= MeshKey::COLOR;
        }
        if self.normals.is_some() {
            variant |= MeshKey::NORMAL;
        }
        if self.uvs.is_some() {
            variant |= MeshKey::UV;
        }
        variant
    }

    /// Clears all buffers.
    pub fn clear(&mut self) {
        self.indices.clear();
        self.positions.clear();
        if let Some(colors) = &mut self.colors {
            colors.clear();
        }
        if let Some(normals) = &mut self.normals {
            normals.clear();
        }
        if let Some(uvs) = &mut self.uvs {
            uvs.clear();
        }
    }

    /**
     * Interleaves vertex data into a single packed byte array.
     */
    fn vertex_bytes(&self) -> Vec<u8> {
        self.check_vertices();
        let mut vertex_data = Vec::with_capacity(self.vertex_count() * self.vertex_size());
        let positions = &self.positions;
        for i in 0..positions.len() {

            // Position
            let bytes = bytes_of(&self.positions[i]);
            vertex_data.extend_from_slice(bytes);

            // Colors
            if let Some(colors) = &self.colors {
                let bytes = bytes_of(&colors[i]);
                vertex_data.extend_from_slice(bytes);
            }

            // Normals
            if let Some(normals) = &self.normals {
                let bytes = bytes_of(&normals[i]);
                vertex_data.extend_from_slice(bytes);
            }

            // UVs
            if let Some(uvs) = &self.uvs {
                let bytes = bytes_of(&uvs[i]);
                vertex_data.extend_from_slice(bytes);
            }
        }
        vertex_data
    }

    fn index_bytes(&self) -> &[u8] {
        &bytemuck::cast_slice(&self.indices)
    }

    /// Number of vertices stored.
    fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Size of each vertex in bytes.
    fn vertex_size(&self) -> usize {
        let mut size = MeshData::POSITION_SIZE;
        if self.colors.is_some() {
            size += MeshData::COLOR_SIZE;
        }
        if self.normals.is_some() {
            size += MeshData::NORMAL_SIZE;
        }
        if self.uvs.is_some() {
            size += MeshData::UV_SIZE;
        }
        size
    }

    // Checks that vertex buffers all have the same length.
    fn check_vertices(&self) {
        let num_vertices = self.positions.len();
        if let Some(colors) = &self.colors {
            if colors.len() != num_vertices {
                panic!("Color buffer had an different length");
            }
        }
        if let Some(normals) = &self.normals {
            if normals.len() != num_vertices {
                panic!("Normal buffer had an different length");
            }
        }
        if let Some(uvs) = &self.uvs {
            if uvs.len() != num_vertices {
                panic!("UV buffer had an different length");
            }
        }
    }
}

bitflags! {
    /// Determines the "permutation" of a mesh.
    /// These are flags that determine which vertex attributes are available in a given mesh.
    /// Used for selecting pipelines from a cache.
    #[derive(Copy, Clone, Eq, PartialEq, Default, Debug, Hash)]
    pub struct MeshKey: u8 {
        const NONE      = 0b00000000;
        const COLOR     = 0b00000001;
        const NORMAL    = 0b00000010;
        const UV        = 0b00000100;
        const ALL       = 0b11111111;
    }
}

impl MeshKey {
    /**
     * Gets mesh data necessary to build a pipeline.
     */
    pub fn layout(self, defs: &mut ShaderPreprocessor) -> MeshLayout {
        
        // Position
        let mut layout = MeshLayout::default();
        let mut offset = 0;
        layout.attributes.push(VertexAttribute {
            format: VertexFormat::Float32x3,
            offset,
            shader_location: MeshData::POSITION_LOCATION,
        });
        offset += MeshData::POSITION_SIZE as u64;

        // Color
        if self & Self::COLOR != Self::NONE {
            layout.attributes.push(VertexAttribute {
                format: VertexFormat::Float32x4,
                offset,
                shader_location: MeshData::COLOR_LOCATION,
            });
            offset += MeshData::COLOR_SIZE as u64;
            defs.add("COLOR");
        }

        // Normal
        if self & Self::NORMAL != Self::NONE {
            layout.attributes.push(VertexAttribute {
                format: VertexFormat::Float32x3,
                offset,
                shader_location: MeshData::NORMAL_LOCATION,
            });
            offset += MeshData::NORMAL_SIZE as u64;
            defs.add("NORMAL");
        }

        // UV
        if self & Self::UV != Self::NONE {
            layout.attributes.push(VertexAttribute {
                format: VertexFormat::Float32x2,
                offset,
                shader_location: MeshData::UV_LOCATION,
            });
            offset += MeshData::UV_SIZE as u64;
            defs.add("UV");
        }
        layout.array_stride = offset;
        layout
    }
}

/// Similar to a [`VertexBufferLayout`], but attributes are stored in a Vec rather than a slice.
/// Needed for generating layouts dynamically.
#[derive(Default, Debug)]
pub struct MeshLayout {
    array_stride: u64,
    attributes: Vec<VertexAttribute>,
}

impl MeshLayout {
    pub fn as_vertex_layout(&self) -> VertexBufferLayout<'_> {
        VertexBufferLayout {
            array_stride: self.array_stride,
            step_mode: VertexStepMode::Vertex,
            attributes: &self.attributes,
        }
    }
}

/// GPU representation of [`Mesh`].
pub struct Mesh {
    pub(crate) vertices: Buffer,
    pub(crate) indices: Buffer,
    pub(crate) index_format: IndexFormat,
    pub(crate) num_indices: u32,
    pub(crate) key: MeshKey,
}
impl Asset for Mesh {}

impl Mesh {
    pub fn from_data(mesh: &MeshData, device: &Device) -> Self {
        Self {
            vertices: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: &mesh.vertex_bytes(),
                usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
            }),
            indices: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("index_buffer"),
                contents: mesh.index_bytes(),
                usage: BufferUsages::COPY_DST | BufferUsages::INDEX,
            }),
            index_format: IndexFormat::Uint32,
            num_indices: mesh.indices.len() as u32,
            key: mesh.key(),
        }
    }
}