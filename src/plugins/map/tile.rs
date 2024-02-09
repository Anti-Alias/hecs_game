use super::Properties;

#[derive(Clone, Default, Debug)]
pub struct Tile {
    /// ID of tile local to its tileset
    pub id: u32,
    pub properties: Properties,
}

/// Global tile id
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug, Hash, Ord, PartialOrd)]
pub struct Gid {
    pub tileset_index: u32,
    pub tilde_id: u32,
}