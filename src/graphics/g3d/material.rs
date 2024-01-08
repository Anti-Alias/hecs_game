use crate::Color;

pub struct Material {
    pub base_color: Color,
}

impl From<Color> for Material {
    fn from(base_color: Color) -> Self {
        Self { base_color }
    }
}