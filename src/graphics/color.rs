use bytemuck::{Pod, Zeroable};

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