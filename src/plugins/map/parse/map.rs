use roxmltree::{Document, Node};
use crate::map::{Layer, Orientation, RenderOrder, TmxParseError};
use crate::map::parse;

/// A mostly 1:1 mapping of the TMX <map> specification, with dependent tileset left unloaded.
#[derive(Debug)]
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

impl Default for TiledMap {
    fn default() -> Self {
        Self {
            version: Default::default(),
            orientation: Default::default(),
            render_order: Default::default(),
            width: Default::default(),
            height: Default::default(),
            tile_width: Default::default(),
            tile_height: Default::default(),
            chunk_width: 16,
            chunk_height: 16,
            tilesets: Default::default(),
            infinite: Default::default(),
            layers: Default::default()
        }
    }
}

impl TiledMap {

    pub fn parse_doc(map_doc: Document, parent_path: Option<&str>) -> Result<Self, TmxParseError> {
        let mut map = Self::default();
        let root = map_doc.root();
        for node in root.children() {
            let tag_name = node.tag_name().name();
            match tag_name {
                "map" => map.parse(node, parent_path)?,
                _ => {},
            }
        }

        Ok(map)
    }

    fn parse(&mut self, map_node: Node, parent_path: Option<&str>) -> Result<(), TmxParseError> {

        // Parses map attributes
        for attribute in map_node.attributes() {
            let name = attribute.name();
            let value = attribute.value();
            match name {
                "version" => self.version = String::from(value),
                "orientation" => self.orientation = Orientation::from_str(value)?,
                "renderorder" => self.render_order = RenderOrder::from_str(value)?,
                "width" => self.width = value.parse()?,
                "height" => self.height = value.parse()?,
                "tilewidth" => self.tile_width = value.parse()?,
                "tileheight" => self.tile_height = value.parse()?,
                "infinite" => self.infinite = match value {
                    "0" => false,
                    "1" => true,
                    _ => return Err(TmxParseError::InvalidAttributeValue { value: String::from(value) }),
                },
                _ => {}
            }
        }
    
        // Traverses children
        for node in map_node.children() {
            let tag_name = node.tag_name().name();
            match tag_name {
                "tileset" => self.tilesets.push(TilesetEntry::parse(node, parent_path)?),
                _ => {},
            }
        }

        Ok(())
    }
}

/// A single tileset stored in a [`TiledMap`]`.
/// Either stores the tileset, or references it in another file.
#[derive(Clone, Debug)]
pub enum TilesetEntry {
    Internal {
        first_gid: u32,
        tileset: parse::Tileset,
    },
    External {
        first_gid: u32,
        source: String,
    }
}

impl TilesetEntry {
    fn parse(entry_node: Node, parent_path: Option<&str>) -> Result<Self, TmxParseError> {
        let first_gid: u32 = entry_node
            .attributes()
            .find(|attr| attr.name() == "firstgid")
            .ok_or(TmxParseError::MissingTagError { tag_name: String::from("firstgid") })?
            .value()
            .parse()?;
        let source: Option<&str> = entry_node
            .attributes()
            .find(|attr| attr.name() == "source")
            .map(|attr| attr.value());

        if let Some(source) = source {
            let source = match parent_path {
                Some(parent_path) => format!("{parent_path}/{source}"),
                None => String::from(source),
            };
            Ok(TilesetEntry::External { first_gid, source })
        }
        else {
            let mut tileset = parse::Tileset::default();
            tileset.parse(entry_node, parent_path)?;
            Ok(TilesetEntry::Internal { first_gid, tileset })
        }
    }
}

