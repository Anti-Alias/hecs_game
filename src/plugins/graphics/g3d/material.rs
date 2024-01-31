use crate::{Asset, AssetStorage, Color, Handle, ShaderPreprocessor, Texture};
use bitflags::bitflags;
use bytemuck::cast_slice;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BufferBinding, BufferBindingType, BufferUsages, Device, Face, SamplerBindingType, ShaderStages, TextureSampleType, TextureViewDimension};


#[derive(Default)]
pub struct Material {
    pub base_color: Color,
    pub base_color_texture: Option<Handle<Texture>>,
    pub cull_mode: Option<Face>,
    pub prepared: Option<PreparedMaterial>,
}

impl Material {

    const UNIFORM_BINDING: u32 = 0;
    const BASE_COLOR_TEX_BINDING: u32 = 1;
    const BASE_COLOR_SAM_BINDING: u32 = 2;

    pub fn uniform_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(&self.base_color)
    }

    /// Returns prepared material if all dependent textures are loaded.
    pub(crate) fn prepare<'a>(&'a mut self, textures: &AssetStorage<Texture>, device: &Device) {

        if self.prepared.is_some() {
            return;
        }
        if !is_tex_loaded(&self.base_color_texture, textures) {
            return;
        }

        // Color buffer
        let color = &[self.base_color];
        let uniform_bytes: &[u8] = cast_slice(color);
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: uniform_bytes,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });

        // Bind group / layout
        let mut layout_entries = Vec::new();
        let mut group_entries = Vec::new();
        let mut flags = MaterialFlags::NONE;

        // Base color
        layout_entries.push(BindGroupLayoutEntry {
            binding: Self::UNIFORM_BINDING,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        group_entries.push(BindGroupEntry {
            binding: Self::UNIFORM_BINDING,
            resource: BindingResource::Buffer(BufferBinding {
                buffer: &uniform_buffer,
                offset: 0,
                size: None,
            }),
        });

        // Base color texture
        if let Some(base_color_texture) = &self.base_color_texture {
            let base_color_texture = textures.get(base_color_texture);
            let base_color_texture = base_color_texture.unwrap();
            let entries = base_color_texture.create_entries(Self::BASE_COLOR_TEX_BINDING, Self::BASE_COLOR_SAM_BINDING);
            layout_entries.push(entries.layout_texture_entry);
            layout_entries.push(entries.layout_sampler_entry);
            group_entries.push(entries.group_texture_entry);
            group_entries.push(entries.group_sampler_entry);
            flags |= MaterialFlags::BASE_COLOR_TEX;
        }

        // Finishes preparing material
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &layout_entries,
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &group_entries,
        });
        self.prepared = Some(PreparedMaterial {
            key: MaterialKey { flags, cull_mode: self.cull_mode },
            bind_group_layout,
            bind_group,
        });
    }
}
impl Asset for Material {}

pub fn is_tex_loaded(texture: &Option<Handle<Texture>>, textures: &AssetStorage<Texture>) -> bool {
    if let Some(texture) = texture {
        if !textures.get(texture).is_loaded() {
            return false;
        }
    }
    true
}


/// Material data that has been "prepared" for use in the graphics engine.
pub struct PreparedMaterial {
    pub key: MaterialKey,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
}

impl PreparedMaterial {
    pub fn write_shader_defs(&self, defs: &mut ShaderPreprocessor) {
        let flags = self.key.flags;
        if flags & MaterialFlags::BASE_COLOR_TEX != MaterialFlags::NONE {
            defs.add("BASE_COLOR_TEX");
        }
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
            binding: Material::UNIFORM_BINDING,
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