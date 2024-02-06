use crate::AssetPath;

/**
 * A method of receiving bytes from files.
 * IE: file, http, https, etc.
 */
pub trait Protocol: Send + Sync + 'static {
    /**
     * Name of the protocol. IE: file, http, https etc.
     * Should not change across invocations.
     */
    fn name(&self) -> &str;
    /**
     * Retrieves raw bytes from the path specified.
     */
    fn read(&self, path: &AssetPath) -> anyhow::Result<Vec<u8>>;
}

/**
 * An implementation of [`Protocol`] that fetches bytes from the file system.
 */
#[derive(Copy, Clone, Debug)]
pub struct FileProtocol;
impl Protocol for FileProtocol {
    fn name(&self) -> &str { return "file" }
    fn read(&self, path: &AssetPath) -> anyhow::Result<Vec<u8>> {
        let bytes = std::fs::read(path.without_protocol())?;
        Ok(bytes)
    }
}

/**
 * An implementation of [`Protocol`] that always returns the bytes it stores.
 * Useful for testing purposes.
 */
#[derive(Clone, Debug)]
pub struct RawProtocol(pub &'static [u8]);
impl From<&'static str> for RawProtocol {
    fn from(value: &'static str) -> Self {
        Self(value.as_bytes())
    }
}
impl Protocol for RawProtocol {
    fn name(&self) -> &str { return "raw" }
    fn read(&self, _path: &AssetPath) -> anyhow::Result<Vec<u8>> {
        Ok(self.0.to_vec())
    }
}