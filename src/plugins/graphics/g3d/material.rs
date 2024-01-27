use crate::{Color, Handle, Texture};
use bitflags::bitflags;
use wgpu::util::{DeviceExt, BufferInitDescriptor};
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BufferBinding, BufferUsages, Device, Face, SamplerBindingType, ShaderStages, TextureSampleType, TextureViewDimension};


#[derive(Clone, Default)]
pub struct Material {
    pub base_color: Color,
    pub base_color_texture: Option<Handle<Texture>>,
    pub cull_mode: Option<Face>,
}

impl Material {

    const BASE_COLOR_BINDING: u32 = 0;
    const BASE_COLOR_TEX_BINDING: u32 = 1;
    const BASE_COLOR_SAM_BINDING: u32 = 2;

    pub fn uniform_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(&self.base_color)
    }

    pub fn key(&self) -> MaterialKey {
        let mut flags = MaterialFlags::NONE;
        if self.base_color_texture.is_some() {
            flags |= MaterialFlags::BASE_COLOR_TEX;
        }
        MaterialKey { flags, cull_mode: self.cull_mode }
    }
}

pub struct MaterialLayout(Vec<BindGroupLayoutEntry>);

/// Info about materal used in pipeline selection.
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug, Hash)]
pub struct MaterialKey {
    pub flags: MaterialFlags,
    pub cull_mode: Option<Face>,
}

impl MaterialKey {
    pub fn layout(&self) -> MaterialLayout {
        
        // Base color
        let mut layout = vec![BindGroupLayoutEntry {
            binding: Material::BASE_COLOR_BINDING,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];

        if self.flags & MaterialFlags::BASE_COLOR_TEX != MaterialFlags::NONE {

            // Base color texture
            layout.push(BindGroupLayoutEntry {
                binding: Material::BASE_COLOR_TEX_BINDING,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::default(),
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });

            // Base color sampler
            layout.push(BindGroupLayoutEntry {
                binding: Material::BASE_COLOR_SAM_BINDING,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            });
        }
        MaterialLayout(layout)
    }
}

bitflags! {
    /// Determines the "permutation" of a mesh.
    /// These are flags that determine which vertex attributes are available in a given mesh.
    /// Used for selecting pipelines from a cache.
    #[derive(Copy, Clone, Eq, PartialEq, Default, Debug, Hash)]
    pub struct MaterialFlags: u8 {
        const NONE              = 0b00000000;
        const BASE_COLOR_TEX    = 0b00000001;
        const ALL               = 0b11111111;
    }
}


/// GPU representation of [`Material`].
pub struct GpuMaterial {
    pub(crate) bind_group: BindGroup,
    pub(crate) layout: BindGroupLayout,
    pub (crate) key: MaterialKey,
}

impl GpuMaterial {
    pub fn from_material(material: &Material, device: &Device) -> Self {
        let key = material.key();

        // Uploads uniform entry
        let uniform_data = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: material.uniform_bytes(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        // Builds bind group and bind group layout entries
        let mut layout_entries = Vec::new();
        let mut group_entries = Vec::new();

        // Base color
        layout_entries.push(BindGroupLayoutEntry {
            binding: Material::BASE_COLOR_BINDING,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        group_entries.push(BindGroupEntry {
            binding: Material::BASE_COLOR_BINDING,
            resource: BindingResource::Buffer(BufferBinding {
                buffer: &uniform_data,
                offset: 0,
                size: None,
            }),
        });

        // Base color texture and sampler
        if key.flags & MaterialFlags::BASE_COLOR_TEX != MaterialFlags::NONE {
            layout_entries.push(BindGroupLayoutEntry {
                binding: Material::BASE_COLOR_TEX_BINDING,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::default(),
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
            layout_entries.push(BindGroupLayoutEntry {
                binding: Material::BASE_COLOR_SAM_BINDING,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            });
            group_entries.push(BindGroupEntry {
                binding: Material::BASE_COLOR_TEX_BINDING,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &uniform_data,
                    offset: 0,
                    size: None,
                }),
            });
        }

        // Creates layout for material
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("material_layout"),
            entries: &layout_entries,
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("material_bind_group"),
            layout: &layout,
            entries: &group_entries,
        });



        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("material"),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: Material::BASE_COLOR_BINDING,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &uniform_data,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        Self { bind_group, layout, key }
    }
}