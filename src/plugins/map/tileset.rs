use roxmltree::Document;
use crate::map::parse;
use crate::{AssetManager, AssetValue, Handle, HashMap, Readiness, Texture};
use crate::{Asset, AssetLoader, AssetPath, AssetResult, map::TmxParseError};
use super::{Orientation, Tile};

/// Loader for a .tsx file.
/// Outputs a [`Tileset`].
pub struct TsxLoader;
impl AssetLoader for TsxLoader {
    type AssetType = Tileset;

    fn load(&self, bytes: &[u8], path: &AssetPath) -> AssetResult<Tileset> {
        let xml_source = std::str::from_utf8(bytes)?;
        let xml_doc = Document::parse(xml_source)?;
        let parsed_tileset = parse::Tileset::parse_doc(xml_doc, path.parent().as_deref())?;
        Ok(AssetValue::from_fn(|manager| {
            Tileset::from_parsed(parsed_tileset, manager)
        }))
    }

    fn extensions(&self) -> &[&str] {
        &["tsx"]
    }
}

/// A processed version of [`parse::Tileset`] such that images are represented as handles.
#[derive(Clone, Default, Debug)]
pub struct Tileset {
    pub name: String,
    pub class: String,
    pub tile_width: u32,
    pub tile_height: u32,
    pub spacing: u32,
    pub margin: u32,
    pub tile_count: u32,
    pub columns: u32,
    pub object_alignment: ObjectAlignment,
    pub tile_render_size: TileRenderSize,
    pub fill_mode: FillMode,
    pub tile_offset: Option<TileOffset>,
    pub grid: Option<Grid>,
    pub image: Option<Handle<Texture>>,
    pub tiles: HashMap<u32, Tile>,
}

impl Tileset {
    pub fn from_parsed(parsed_tileset: parse::Tileset, manager: &AssetManager) -> Self {
        let image = parsed_tileset.image.map(|parsed_image| {
            manager.load(parsed_image.source)
        });
        Self {
            name: parsed_tileset.name,
            class: parsed_tileset.class,
            tile_width: parsed_tileset.tile_width,
            tile_height: parsed_tileset.tile_height,
            spacing: parsed_tileset.spacing,
            margin: parsed_tileset.margin,
            tile_count: parsed_tileset.tile_count,
            columns: parsed_tileset.columns,
            object_alignment: parsed_tileset.object_alignment,
            tile_render_size: parsed_tileset.tile_render_size,
            fill_mode: parsed_tileset.fill_mode,
            tile_offset: parsed_tileset.tile_offset,
            grid: parsed_tileset.grid,
            image,
            tiles: parsed_tileset.tiles,
        }
    }
}

impl Asset for Tileset {
    fn readiness(&self, assets: &AssetManager) -> crate::Readiness {
        match self.image.as_ref() {
            Some(image_handle) => assets.readiness_of(image_handle),
            None => Readiness::Ready,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub enum ObjectAlignment {
    #[default]
    Unspecified,
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub enum FillMode {
    #[default]
    Stretch,
    PreserveAspectFit,
}

impl FillMode {
    pub fn parse(str: &str) -> Result<Self, TmxParseError> {
        match str {
            "stretch" => Ok(Self::Stretch),
            "preserve-aspect-fit" => Ok(Self::PreserveAspectFit),
            _ => Err(TmxParseError::InvalidAttributeValue { value: String::from(str) })
        }
    }
}

impl ObjectAlignment {
    pub fn parse(str: &str) -> Result<Self, TmxParseError> {
        match str {
            "unspecified" => Ok(Self::Unspecified),
            "topleft" => Ok(Self::TopLeft),
            "top" => Ok(Self::Top),
            "topright" => Ok(Self::TopRight),
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            "bottomleft" => Ok(Self::BottomLeft),
            "bottom" => Ok(Self::Bottom),
            "bottomright" => Ok(Self::BottomRight),
            _ => Err(TmxParseError::InvalidAttributeValue { value: String::from(str) })
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub enum TileRenderSize {
    #[default]
    Tile,
    Grid,
}

impl TileRenderSize {
    pub fn parse(str: &str) -> Result<Self, TmxParseError> {
        match str {
            "tile" => Ok(Self::Tile),
            "grid" => Ok(Self::Grid),
            _ => Err(TmxParseError::InvalidAttributeValue { value: String::from(str) })
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct TileOffset { pub x: i32, pub y: i32 }

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct Grid {
    pub orientation: Orientation,
    pub width: u32,
    pub height: u32,
}