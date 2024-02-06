use crate::{Asset, Handle, TmxParseError};

#[derive(Clone, Debug, Default)]
pub struct TiledMap {
    pub(crate) version: String,
    pub(crate) orientation: Orientation,
    pub(crate) render_order: RenderOrder,
    pub(crate) width: u32, 
    pub(crate) height: u32,
    pub(crate) tile_width: u32,
    pub(crate) tile_height: u32,
    pub(crate) tilesets: Vec<Handle<Tileset>>,
    pub(crate) infinite: bool,
}

impl Asset for TiledMap {}

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub enum RenderOrder {
    #[default]
    RightDown,
    RightUp,
    LeftDown,
    LeftUp,
}

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub enum Orientation {
    #[default]
    Orthogonal,
    Isometric,
    Staggered,
}

impl Orientation {
    pub fn from_str(value: &str) -> Result<Self, TmxParseError> {
        match value {
            "orthogonal" => Ok(Self::Orthogonal),
            "isometric" => Ok(Self::Isometric),
            "staggered" => Ok(Self::Staggered),
            _ => Err(TmxParseError::InvalidAttributeValue { value: String::from(value) })
        }
    }
}

impl RenderOrder {
    pub fn from_str(value: &str) -> Result<Self, TmxParseError> {
        match value {
            "right-down" => Ok(Self::RightDown),
            "right-up" => Ok(Self::RightUp),
            "left-down" => Ok(Self::LeftDown),
            "left-up" => Ok(Self::LeftUp),
            _ => Err(TmxParseError::InvalidAttributeValue { value: String::from(value) })
        }
    }
}

pub struct Tileset {}
impl Asset for Tileset {}