use std::sync::Arc;

use crate::{AssetManager, DynHandle, HandleId};

/**
 * Wrapper around an [`AssetManager`].
 * All assets loaded from it will be considered 
 */
#[derive(Clone)]
pub struct AssetBatch {
    pub(crate) assets: AssetManager,
    pub(crate) handles: Arc<std::sync::RwLock<Vec<HandleId>>>,
}