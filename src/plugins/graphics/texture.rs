use std::sync::Arc;
use wgpu::{Device, Queue};
use crate::{Dependencies, Loader};

pub struct TextureLoader {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
}

impl Loader for TextureLoader {

    type AssetType = Texture;
    const EXTENSIONS: &'static [&'static str] = &["png", "jpg", "jpeg"];

    fn load(&self, bytes: &[u8], extension: &str, dependencies: Dependencies) -> anyhow::Result<Self::AssetType> {
        println!("Extension: {extension}");
        todo!()
    }
}


/// A texture with an associated sampler.
pub struct Texture {
    pub texture: wgpu::Texture,
    pub sampler: wgpu::Sampler,
}