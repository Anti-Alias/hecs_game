use crate::Color;
use bitflags::bitflags;
use wgpu::util::{DeviceExt, BufferInitDescriptor};
use wgpu::{BindGroup, Device, BindGroupLayout, BindGroupDescriptor, BindGroupLayoutDescriptor, BindGroupLayoutEntry, ShaderStages, BindingType, BindGroupEntry, BindingResource, BufferBinding, BufferUsages};


pub struct Material {
    pub base_color: Color,
}

impl Material {
    const UNIFORM_BINDING: u32 = 0;
    pub fn variant(&self) -> MaterialVariant {
        MaterialVariant::NONE
    }
    pub fn uniform_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(&self.base_color)
    }
}

bitflags! {
    /// Determines the "permutation" of a mesh.
    /// These are flags that determine which vertex attributes are available in a given mesh.
    /// Used for selecting pipelines from a cache.
    #[derive(Copy, Clone, Eq, PartialEq, Default, Debug, Hash)]
    pub struct MaterialVariant: u8 {
        const NONE      = 0b00000000;
        const ALL       = 0b11111111;
    }
}


/// GPU representation of [`Material`].
pub struct GpuMaterial {
    pub(crate) bind_group: BindGroup,
    pub(crate) layout: BindGroupLayout,
    pub (crate) variant: MaterialVariant,
}

impl GpuMaterial {
    pub fn from_material(material: &Material, device: &Device) -> Self {
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("material_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: Material::UNIFORM_BINDING,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: material.uniform_bytes(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("material"),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: Material::UNIFORM_BINDING,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        let variant = material.variant();
        Self { bind_group, layout, variant }
    }
}