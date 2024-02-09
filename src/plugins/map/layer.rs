use derive_more::*;
use crate::HashMap;
use super::{Gid, Properties, TiledMap};

#[derive(Debug)]
pub struct Layer {
    pub properties: Properties,
    pub kind: LayerKind,
}

#[derive(Debug)]
pub enum LayerKind {
    TileLayer(TileLayer),
    GroupLayer(GroupLayer),
}

#[derive(Debug)]
pub struct TileLayer {
    pub width: u32,
    pub height: u32,
    pub kind: TileLayerKind,
}

#[derive(Debug)]
pub enum TileLayerKind {
    FiniteTileLayer(FiniteTileLayer),
    InfiniteTileLayer(InfiniteTileLayer),
}

impl TileLayer {
    pub fn get_tile_gid(&self, x: i32, y: i32, map: &TiledMap) -> Option<Gid> {
        match &self.kind {
            TileLayerKind::FiniteTileLayer(layer) => {
                let x = x as usize;
                let y = y as usize;
                let width = map.width as usize;
                let idx = y*width + x;
                layer.get(idx).map(|gid| *gid)
            },
            TileLayerKind::InfiniteTileLayer(layer) => {
                let (chunk_x, local_x) = to_chunk_coords(x, map.chunk_width);
                let (chunk_y, local_y) = to_chunk_coords(y, map.chunk_height);
                let chunk = layer.get(&(chunk_x, chunk_y))?;
                let local_idx = local_y * map.chunk_width as i32 + local_x;
                chunk.get(local_idx as usize).map(|gid| *gid)
            }
        }
    }

    /// Computes minx, miny, maxx and maxy of tiles
    pub fn bounds(&self, map: &TiledMap) -> (i32, i32, i32, i32) {
        match &self.kind {
            TileLayerKind::FiniteTileLayer(_) => {
                (0, 0, self.width as i32, self.height as i32)
            },
            TileLayerKind::InfiniteTileLayer(layer) => {
                let mut min_x = i32::MAX;
                let mut min_y = i32::MAX;
                let mut max_x = i32::MIN;
                let mut max_y = i32::MIN;
                let chunk_width = map.chunk_width as i32;
                let chunk_height = map.chunk_height as i32;
                for (chunk_x, chunk_y) in layer.0.keys().copied() {
                    let cmin_x = chunk_x * chunk_width;
                    let cmin_y = chunk_y * chunk_height;
                    let cmax_x = cmin_x + chunk_width;
                    let cmax_y = cmin_y + chunk_height;
                    min_x = min_x.min(cmin_x);
                    min_y = min_y.min(cmin_y);
                    max_x = max_x.max(cmax_x);
                    max_y = max_y.min(cmax_y);
                }
                (min_x, min_y, max_x, max_y)
            },
        }
    }
}

fn to_chunk_coords(v: i32, size: u32) -> (i32, i32) {
    let size = size as i32;
    if v >= 0 {
        (v / size, v % size)
    }
    else {
        ((v-1) / size, (size + v) % size)
    }
}

/// Vec to global tile ids
#[derive(Debug, Deref)]
pub struct FiniteTileLayer(Vec<Gid>);

/// Chunks of tile ids
#[derive(Debug, Deref)]
pub struct InfiniteTileLayer(HashMap<(i32, i32), Vec<Gid>>);

#[derive(Debug, Deref)]
pub struct GroupLayer(Vec<Layer>);
impl GroupLayer {
    pub fn iter(&self) -> impl Iterator<Item = &Layer> {
        self.0.iter()
    }
}