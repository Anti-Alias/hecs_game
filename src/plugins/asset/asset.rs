use std::any::Any;

use crate::HandleStatus;

/**
 * Shareable resource like an image, animation, sound etc.
 */
pub trait Asset: Any + Send + Sync + 'static {
    /**
     * The merged [`HandleStatus`] of all dependencies, if any.
     * If at least one failed, status is [`HandleStatus::Failed`].
     * If at least one is loading and none are failed, status is [`HandleStatus::Loading`].
     * If all are loaded, status is loaded [`HandleStatus::Loaded`].
     */
    fn status(&self) -> HandleStatus {
        return HandleStatus::Loaded
    }
}
impl<A: Send + Sync + 'static> Asset for A {}