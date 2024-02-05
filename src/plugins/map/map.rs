use crate::{Asset, AssetLoader, AssetValue, Handle, Tileset};
use derive_more::*;
use roxmltree::{Document, Node};


#[derive(Clone, Default)]
pub struct TiledMap {
    pub width: u32, 
    pub height: u32,
    pub tilewidth: u32,
    pub tileheight: u32,
    pub tilesets: Vec<Handle<Tileset>>,
}

struct TmxLoader;
impl AssetLoader for TmxLoader {

    type AssetType = TiledMap;

    fn load(&self, bytes: &[u8], _path: &crate::AssetPath) -> anyhow::Result<AssetValue<Self::AssetType>> {
        let mut map = TiledMap::default();
        let source = std::str::from_utf8(bytes)?;
        let doc = Document::parse(source)?;
        let root = doc.root();
        for node in root.children() {
            let tag_name = node.tag_name().name();
            match tag_name {
                "map" => parse_map_node(&mut map, node),
                _ => Err(TiledMapError::UnexpectedTagError { tag_name: tag_name.into() })
            }?
        }
        Ok(AssetValue::Asset(map))
    }

    fn extensions(&self) -> &[&str] {
        &["tmx"]
    }
}


fn parse_map_node(map: &mut TiledMap, map_node: Node) -> Result<(), TiledMapError> {
    for node in map_node.descendants() {
        let tag_name = node.tag_name().name();
        match tag_name {
            "tileset" => {},
            _ => {},

        }
    }
    Ok(())
}

impl Asset for TiledMap {}


#[derive(Error, Display, From, Debug)]
pub enum TiledMapError {
    XmlError(roxmltree::Error),
    #[display(fmg="Unexpected tag '{tag}'")]
    UnexpectedTagError { tag_name: String },
}

#[cfg(test)]
mod test {
    use super::TmxLoader;

    #[test]
    fn test() {
        let loader = TmxLoader;
    }
}