use std::collections::HashMap;
use std::num::ParseIntError;
use crate::{AssetManager, Color, Readiness};
use crate::{Asset, AssetLoader, AssetResult, AssetValue, Handle, map::Tileset};
use crate::map::parse;
use roxmltree::Document;
use derive_more::*;

use super::Layer;

/// [`AssetLoader`] for a [`TiledMap`] coming from a tmx file.
pub struct TmxLoader;
impl AssetLoader for TmxLoader {
    type AssetType = TiledMap;

    fn load(&self, bytes: &[u8], path: &crate::AssetPath) -> AssetResult<TiledMap> {
        let xml_source = std::str::from_utf8(bytes)?;
        let xml_doc = Document::parse(xml_source)?;
        let parsed_map = parse::TiledMap::parse_doc(xml_doc, path.parent().as_deref())?;
        Ok(AssetValue::from_fn(|manager| {
            TiledMap::from_parsed(parsed_map, manager)
        }))
    }

    fn extensions(&self) -> &[&str] {
        &["tmx"]
    }
}

/// A processed version of [`parse::TiledMap`] such that tilesets are represented as handles.
#[derive(Debug, Default)]
pub struct TiledMap {
    pub version: String,
    pub orientation: Orientation,
    pub render_order: RenderOrder,
    pub width: u32, 
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub chunk_width: u32,
    pub chunk_height: u32,
    pub tilesets: Vec<TilesetEntry>,
    pub infinite: bool,
    pub layers: Vec<Layer>,
}

impl TiledMap {
    fn from_parsed(parsed: parse::TiledMap, manager: &AssetManager) -> Self {
        let tilesets: Vec<TilesetEntry> = parsed.tilesets
            .into_iter()
            .map(|parsed_entry| TilesetEntry::from_parsed(parsed_entry, manager))
            .collect();
        Self {
            version: parsed.version,
            orientation: parsed.orientation,
            render_order: parsed.render_order,
            width: parsed.width,
            height: parsed.height,
            tile_width: parsed.tile_width,
            tile_height: parsed.tile_height,
            chunk_width: parsed.chunk_width,
            chunk_height: parsed.chunk_height,
            tilesets,
            infinite: parsed.infinite,
            layers: parsed.layers,
        }
    }
}

impl Asset for TiledMap {
    fn readiness(&self, assets: &AssetManager) -> Readiness {
        let tileset_handles = self.tilesets
            .iter()
            .map(|entry| &entry.tileset);
        assets.readiness_all(tileset_handles)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub enum RenderOrder {
    #[default]
    RightDown,
    RightUp,
    LeftDown,
    LeftUp,
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

/// A processed version of [`parse::TilesetEntry`] such that tilesets are represented as handles.
#[derive(Clone, Debug)]
pub struct TilesetEntry {
    pub first_gid: u32,
    pub tileset: Handle<Tileset>,
}

impl TilesetEntry {
    fn from_parsed(entry: parse::TilesetEntry, manager: &AssetManager) -> Self {
        match entry {
            parse::TilesetEntry::Internal { first_gid, tileset } => Self {
                first_gid,
                tileset: manager.insert(Tileset::from_parsed(tileset, manager)),
            },
            parse::TilesetEntry::External { first_gid, source } => Self {
                first_gid,
                tileset: manager.load(source),
            },
        }
    }
}

/// A set of properties.
#[derive(Clone, Default, Debug)]
pub struct Properties(HashMap<String, PropertyValue>);
impl Properties {
    pub fn get(&self, name: impl AsRef<str>) -> Option<&PropertyValue> {
        self.0.get(name.as_ref())
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum PropertyValue {
    String(String),
    Float(f32),
    Bool(bool),
    Color(Color),
    File(String),
}

impl PropertyValue {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            PropertyValue::String(str) => Some(&str),
            _ => None,
        }
    }
    pub fn as_float(&self) -> Option<f32> {
        match self {
            PropertyValue::Float(float) => Some(*float),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PropertyValue::Bool(bool) => Some(*bool),
            _ => None,
        }
    }
    pub fn as_color(&self) -> Option<Color> {
        match self {
            PropertyValue::Color(color) => Some(*color),
            _ => None,
        }
    }
    pub fn as_file(&self) -> Option<&str> {
        match self {
            PropertyValue::File(file) => Some(&file),
            _ => None,
        }
    }
}

#[derive(Error, Display, From, Debug)]
pub enum TmxParseError {
    XmlError(roxmltree::Error),
    #[display(fmt="{_0}")]
    ParseIntError(ParseIntError),
    #[display(fmt="Unexpected tag '{tag_name}'")]
    #[from(ignore)]
    UnexpectedTagError { tag_name: String },
    #[display(fmt="Unexpected value {value}")]
    #[from(ignore)]
    InvalidAttributeValue { value: String },
    #[display(fmt="Missing tag {tag_name}")]
    MissingTagError { tag_name: String },
    #[display(fmt="Embedded images not supported")]
    EmbeddedImagesNotSupported,
}