use std::io::Cursor;
use std::sync::Arc;
use image::{DynamicImage, ImageFormat};
use wgpu::{AddressMode, Device, Extent3d, FilterMode, ImageCopyTexture, ImageDataLayout, Origin3d, Queue, SamplerDescriptor, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};
use image::io::Reader as ImageReader;
use derive_more::*;
use bytemuck::cast_slice;
use crate::{AssetLoader, AssetPath};

pub struct TextureLoader {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
}

impl AssetLoader for TextureLoader {

    type AssetType = Texture;

    fn load(&self, bytes: &[u8], path: &AssetPath) -> anyhow::Result<Self::AssetType> {
        let format = match ImageFormat::from_extension(&path.extension) {
            Some(format) => Ok(format),
            None => Err(LoadError::UnsupportedFileExtension),
        }?;
        let mut reader = ImageReader::new(Cursor::new(bytes));
        reader.set_format(format);
        let dyn_img = reader.decode()?;
        let tex_data = get_texture_data(dyn_img, true);
        let size = Extent3d {
            width: tex_data.width,
            height: tex_data.height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: tex_data.format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let copy_texture = ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        };
        let layout = ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(tex_data.width * tex_data.format.pixel_size() as u32),
            rows_per_image: None,
        };
        self.queue.write_texture(copy_texture, &tex_data.data, layout, size);
        let sampler = self.device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });
        Ok(Texture { texture, sampler })
    }

    fn extensions(&self) -> &[&str] {
        &["png", "jpg", "jpeg"]
    }
}

struct TextureData {
    data: Vec<u8>,
    width: u32,
    height: u32,
    format: TextureFormat,
}

/// Extracts bytes, image size and texture format from dynamic image.
/// Mostly stolen from Bevy.
/// Thank GOD I don't have to write this...
/// Source: https://github.com/bevyengine/bevy/blob/main/crates/bevy_render/src/texture/image_texture_conversion.rs#L11
fn get_texture_data(dyn_img: DynamicImage, is_srgb: bool) -> TextureData {
    let data;
    let format;
    let width;
    let height;
    match dyn_img {
        DynamicImage::ImageLuma8(image) => {
            let i = DynamicImage::ImageLuma8(image).into_rgba8();
            width = i.width();
            height = i.height();
            format = if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            };

            data = i.into_raw();
        }
        DynamicImage::ImageLumaA8(image) => {
            let i = DynamicImage::ImageLumaA8(image).into_rgba8();
            width = i.width();
            height = i.height();
            format = if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            };

            data = i.into_raw();
        }
        DynamicImage::ImageRgb8(image) => {
            let i = DynamicImage::ImageRgb8(image).into_rgba8();
            width = i.width();
            height = i.height();
            format = if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            };

            data = i.into_raw();
        }
        DynamicImage::ImageRgba8(image) => {
            width = image.width();
            height = image.height();
            format = if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            };

            data = image.into_raw();
        }
        DynamicImage::ImageLuma16(image) => {
            width = image.width();
            height = image.height();
            format = TextureFormat::R16Uint;

            let raw_data = image.into_raw();

            data = cast_slice(&raw_data).to_owned();
        }
        DynamicImage::ImageLumaA16(image) => {
            width = image.width();
            height = image.height();
            format = TextureFormat::Rg16Uint;

            let raw_data = image.into_raw();

            data = cast_slice(&raw_data).to_owned();
        }
        DynamicImage::ImageRgb16(image) => {
            let i = DynamicImage::ImageRgb16(image).into_rgba16();
            width = i.width();
            height = i.height();
            format = TextureFormat::Rgba16Unorm;

            let raw_data = i.into_raw();

            data = cast_slice(&raw_data).to_owned();
        }
        DynamicImage::ImageRgba16(image) => {
            width = image.width();
            height = image.height();
            format = TextureFormat::Rgba16Unorm;

            let raw_data = image.into_raw();

            data = cast_slice(&raw_data).to_owned();
        }
        DynamicImage::ImageRgb32F(image) => {
            width = image.width();
            height = image.height();
            format = TextureFormat::Rgba32Float;

            let mut local_data =
                Vec::with_capacity(width as usize * height as usize * format.pixel_size());

            for pixel in image.into_raw().chunks_exact(3) {
                let r = pixel[0];
                let g = pixel[1];
                let b = pixel[2];
                let a = 1f32;

                local_data.extend_from_slice(&r.to_ne_bytes());
                local_data.extend_from_slice(&g.to_ne_bytes());
                local_data.extend_from_slice(&b.to_ne_bytes());
                local_data.extend_from_slice(&a.to_ne_bytes());
            }

            data = local_data;
        }
        DynamicImage::ImageRgba32F(image) => {
            width = image.width();
            height = image.height();
            format = TextureFormat::Rgba32Float;

            let raw_data = image.into_raw();

            data = cast_slice(&raw_data).to_owned();
        }
        _ => {
            let image = dyn_img.into_rgba8();
            width = image.width();
            height = image.height();
            format = TextureFormat::Rgba8UnormSrgb;

            data = image.into_raw();
        }
    }
    TextureData { data, width, height, format, }
}


/// A texture with an associated sampler.
pub struct Texture {
    pub texture: wgpu::Texture,
    pub sampler: wgpu::Sampler,
}

#[derive(Error, Debug, Display)]
pub enum LoadError {
    #[display(fmt="Unsupported file extension")]
    UnsupportedFileExtension,
}

/// Extends the wgpu [`TextureFormat`] with information about the pixel.
pub trait TextureFormatPixelInfo {
    /// Returns the size of a pixel in bytes of the format.
    fn pixel_size(&self) -> usize;
}

/// Stolen from Bevy.
/// https://github.com/bevyengine/bevy/blob/main/crates/bevy_render/src/texture/image.rs#L789
impl TextureFormatPixelInfo for TextureFormat {
    fn pixel_size(&self) -> usize {
        let info = self;
        match info.block_dimensions() {
            (1, 1) => info.block_size(None).unwrap() as usize,
            _ => panic!("Using pixel_size for compressed textures is invalid"),
        }
    }
}