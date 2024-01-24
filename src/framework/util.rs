use fxhash::{FxHashMap, FxHashSet};

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