use fxhash::{FxHashMap, FxHashSet};
use glam::{UVec2, Vec2};

/**
 * Hash map with a fast non-cryptographically secure hash function.
 */
pub type HashMap<K, V> = FxHashMap<K, V>;

/**
 * Hash map with a fast non-cryptographically secure hash function.
 */
pub type HashSet<V> = FxHashSet<V>;

/**
 * Hash map whose hash function is only suitable for small int types.
 * Outputs the original integer when used.
 */
pub type IntMap<K, V> = identity_hash::IntMap<K, V>;


/// Basic rectangle primitive.
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Rect {
    pub origin: Vec2,
    pub size: Vec2,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Vec2::new(x, y),
            size: Vec2::new(width, height),
        }
    }
}

/// Basic rectangle primitive.
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct URect {
    pub origin: UVec2,
    pub size: UVec2,
}

impl URect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            origin: UVec2::new(x, y),
            size: UVec2::new(width, height),
        }
    }
}

impl From<Rect> for URect {
    fn from(rect: Rect) -> Self {
        Self {
            origin: rect.origin.as_uvec2(),
            size: rect.size.as_uvec2(),
        }
    }
}