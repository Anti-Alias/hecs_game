use roxmltree::{Document, Node};
use crate::map::{FillMode, Grid, ObjectAlignment, TileOffset, TileRenderSize};
use crate::map::TmxParseError;


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
    pub image: Option<Image>,
}

impl Tileset {

    pub fn parse_doc(doc: Document, parent_path: Option<&str>) -> Result<Self, TmxParseError> {
        let mut tileset = Tileset::default();
        let root = doc.root();
        for node in root.children() {
            match node.tag_name().name() {
                "tileset" => tileset.parse(node, parent_path)?,
                _ => {}
            }
        }
        Ok(tileset)
    }

    pub fn parse(&mut self, tileset_node: Node, parent_path: Option<&str>) -> Result<(), TmxParseError> {

        // Parses attributes
        for attribute in tileset_node.attributes() {
            let name = attribute.name();
            let value = attribute.value();
            match name {
                "name" => self.name = String::from(value),
                "class" => self.class = String::from(value),
                "tilewidth" => self.tile_width = value.parse()?,
                "tileheight" => self.tile_height = value.parse()?,
                "spacing" => self.spacing = value.parse()?,
                "margin" => self.margin = value.parse()?,
                "tilecount" => self.tile_count = value.parse()?,
                "columns" => self.columns = value.parse()?,
                "objectalignment" => self.object_alignment = ObjectAlignment::parse(value)?,
                "tilerendersize" => self.tile_render_size = TileRenderSize::parse(value)?,
                "fillmode" => self.fill_mode = FillMode::parse(value)?,
                _ => {}
            }
        }
    
        // Parses children
        for child in tileset_node.children() {
            let tag = child.tag_name().name();
            match tag {
                "image" => self.image = Some(Image::parse(child, parent_path)?),
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Clone, Eq, PartialEq, Default, Debug)]
pub struct Image {
    pub format: String,
    pub source: String,
    pub trans: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl Image {
    pub fn parse(image_node: Node, parent_path: Option<&str>) -> Result<Image, TmxParseError> {
        let mut image = Image::default();
        for attribute in image_node.attributes() {
            let name = attribute.name();
            let value = attribute.value();
            match name {
                "format" => image.format = String::from(value),
                "source" => {
                    let source = match parent_path {
                        Some(parent) => format!("{parent}/{value}"),
                        None => String::from(value),
                    };
                    image.source = source;
                },
                "trans" => image.trans = Some(String::from(value)),
                "width" => image.width = Some(value.parse()?),
                "height" => image.height = Some(value.parse()?),
                _ => {}
            }
        }
        Ok(image)
    }
}