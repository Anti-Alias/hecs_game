use std::any::Any;

use crate::HandleStatus;

/**
 * Shareable resource like an image, animation, sound etc.
 */
pub trait Asset: Any + Send + Sync {
    /**
     * True if all dependencies, if any, are finished loading.
     * If at least one failed, status is [`AssetStatus::Failed`].
     * If at least one is loading and none are failed, status is [`AssetStatus::Loading`].
     * If all are loaded, status is loaded [`AssetStatus::Loaded`].
     */
    fn status(&self) -> HandleStatus {
        return HandleStatus::Loaded
    }
}
impl<A: Send + Sync + 'static> Asset for A {}