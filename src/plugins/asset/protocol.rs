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
    fn read(&self, path: &str) -> anyhow::Result<Vec<u8>>;
}

/**
 * An implementation of [`Protocol`] that fetches bytes from the file system.
 */
#[derive(Copy, Clone, Debug)]
pub struct FileProtocol;
impl Protocol for FileProtocol {
    fn name(&self) -> &str { return "file" }
    fn read(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        println!("File path: {path}");
        let bytes = std::fs::read(path)?;
        Ok(bytes)
    }
}