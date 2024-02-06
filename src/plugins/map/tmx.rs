use std::num::ParseIntError;

use crate::{Asset, AssetLoader, AssetManager, AssetResult, AssetValue, Handle, Orientation, RenderOrder, TiledMap, Tileset};
use derive_more::*;
use roxmltree::{Document, Node};


pub struct TmxLoader;
impl AssetLoader for TmxLoader {

    type AssetType = TiledMap;

    fn load(&self, bytes: &[u8], _path: &crate::AssetPath) -> AssetResult<TiledMap> {

        // Map data to write to
        let mut map = TiledMap::default();
        let mut internal_tilesets = Vec::new();
        let mut external_tilesets = Vec::new();

        // Parses map and writes to above structures
        let source = std::str::from_utf8(bytes)?;
        let doc = Document::parse(source)?;
        let root = doc.root();
        for node in root.children() {
            match node.tag_name().name() {
                "map" => parse_map_node(node, &mut map, &mut internal_tilesets, &mut external_tilesets)?,
                _ => {}
            }
        }
        
        // Finalizes map on main thread
        Ok(AssetValue::from_fn(|manager| {
            let internal_handles = internal_tilesets
                .into_iter()
                .map(|tileset| manager.insert(tileset));
            let external_handles = external_tilesets
                .into_iter()
                .map(|external_tileset| manager.load::<Tileset>(external_tileset.source));
            map.tilesets.extend(internal_handles);
            map.tilesets.extend(external_handles);
            map
        }))
    }

    fn extensions(&self) -> &[&str] {
        &["tmx"]
    }
}

struct ExternalTileset<'a> {
    first_gid: u32,
    source: &'a str,
}


fn parse_map_node(
    map_node: Node,
    map: &mut TiledMap,
    tilesets: &mut Vec<Tileset>,
    external_tilesets: &mut Vec<ExternalTileset>,
) -> Result<(), TmxParseError> {
    for attribute in map_node.attributes() {
        let name = attribute.name();
        let value = attribute.value();
        match name {
            "version" => map.version = String::from(value),
            "orientation" => map.orientation = Orientation::from_str(value)?,
            "renderorder" => map.render_order = RenderOrder::from_str(value)?,
            "width" => map.width = value.parse()?,
            "height" => map.height = value.parse()?,
            "tilewidth" => map.tile_width = value.parse()?,
            "tileheight" => map.tile_height = value.parse()?,
            "infinite" => map.infinite = match value {
                "0" => false,
                "1" => true,
                _ => return Err(TmxParseError::InvalidAttributeValue { value: String::from(value) }),
            },
            _ => {}
        }
    }

    for node in map_node.descendants() {
        let tag_name = node.tag_name().name();
        match tag_name {
            "tileset" => {},
            _ => {},

        }
    }
    Ok(())
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
}