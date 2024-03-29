use bytemuck::{Pod, Zeroable};
use crate::g3d::Material;
use crate::{Texture, Handle};

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug, Pod, Zeroable)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl Color {

    pub const WHITE: Color      = Color::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Color      = Color::new(0.0, 0.0, 0.0, 1.0);
    pub const RED: Color        = Color::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Color      = Color::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Color       = Color::new(0.0, 0.0, 1.0, 1.0);
    pub const YELLOW: Color     = Color::new(1.0, 1.0, 0.0, 1.0);
    pub const TEAL: Color       = Color::new(0.0, 1.0, 1.0, 1.0);
    pub const PINK: Color       = Color::new(1.0, 0.0, 1.0, 1.0);
    pub const GRAY: Color       = Color::new(0.5, 0.5, 0.5, 1.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

impl From<Color> for Material {
    fn from(color: Color) -> Self {
        Material {
            base_color: color,
            ..Default::default()
        }
    }
}

impl From<Handle<Texture>> for Material {
    fn from(base_color_texture: Handle<Texture>) -> Self {
        Material {
            base_color_texture: Some(base_color_texture),
            ..Default::default()
        }
    }
}