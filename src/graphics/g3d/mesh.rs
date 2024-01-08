use std::mem::size_of;
use bytemuck::bytes_of;
use wgpu::{VertexBufferLayout, VertexStepMode, VertexAttribute, VertexFormat, Buffer};
use glam::{Vec3, Vec2};
use bitflags::bitflags;
use crate::{Color, ShaderDefs};

/// Similar to a [`VertexBufferLayout`], but attributes are stored in a Vec rather than a slice.
/// Needed for generating layouts dynamically.
#[derive(Default, Debug)]
pub struct MeshLayout {
    array_stride: u64,
    attributes: Vec<VertexAttribute>,
}

impl MeshLayout {
    pub fn as_wgpu(&self) -> VertexBufferLayout<'_> {
        VertexBufferLayout {
            array_stride: self.array_stride,
            step_mode: VertexStepMode::Vertex,
            attributes: &self.attributes,
        }
    }
}

/**
 * A 3D mesh.
*/
#[derive(Clone, Default)]
pub struct Mesh {
    pub indices:    Vec<u32>,
    pub positions:  Vec<Vec3>,
    pub colors:     Option<Vec<Color>>,
    pub normals:    Option<Vec<Vec3>>,
    pub uvs:        Option<Vec<Vec2>>,
}
impl Mesh {

    const POSITION_LOCATION: u32    = 0;
    const COLOR_LOCATION: u32       = 1;
    const NORMAL_LOCATION: u32      = 2;
    const UV_LOCATION: u32          = 3;

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
    pub fn variant(&self) -> MeshVariant {
        let mut variant = MeshVariant::NONE;
        if self.colors.is_some() {
            variant |= MeshVariant::COLOR;
        }
        if self.normals.is_some() {
            variant |= MeshVariant::NORMAL;
        }
        if self.uvs.is_some() {
            variant |= MeshVariant::UV;
        }
        variant
    }

    /**
     * Interleaves vertex data into a single packed byte array.
     */
    pub fn pack_vertices(&self, vertex_data: &mut Vec<u8>) {
        self.check_vertices();
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
    #[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
    pub struct MeshVariant: u8 {
        const NONE      = 0b00000000;
        const COLOR     = 0b00000001;
        const NORMAL    = 0b00000010;
        const UV        = 0b00000100;
        const ALL       = 0b11111111;
    }
}

impl MeshVariant {
    /**
     * Gets mesh data necessary to build a pipeline.
     */
    pub fn pipeline_data(self, defs: &mut ShaderDefs, layout: &mut MeshLayout) {
        
        // Position
        let mut offset = 0;
        layout.attributes.push(VertexAttribute {
            format: VertexFormat::Float32x3,
            offset,
            shader_location: Mesh::POSITION_LOCATION,
        });
        offset += Mesh::POSITION_SIZE as u64;

        // Color
        if self & Self::COLOR != Self::NONE {
            layout.attributes.push(VertexAttribute {
                format: VertexFormat::Float32x4,
                offset,
                shader_location: Mesh::COLOR_LOCATION,
            });
            offset += Mesh::COLOR_SIZE as u64;
            defs.add("COLOR");
        }

        // Normal
        if self & Self::NORMAL != Self::NONE {
            layout.attributes.push(VertexAttribute {
                format: VertexFormat::Float32x3,
                offset,
                shader_location: Mesh::NORMAL_LOCATION,
            });
            offset += Mesh::NORMAL_SIZE as u64;
            defs.add("NORMAL");
        }

        // UV
        if self & Self::UV != Self::NONE {
            layout.attributes.push(VertexAttribute {
                format: VertexFormat::Float32x2,
                offset,
                shader_location: Mesh::UV_LOCATION,
            });
            offset += Mesh::UV_SIZE as u64;
            defs.add("UV");
        }
        layout.array_stride = offset;
    }
}

pub struct GpuMesh {
    pub vertices: Buffer,
    pub indices: Buffer,
}