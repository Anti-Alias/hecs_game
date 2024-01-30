use std::fmt;

use super::LoadError;

/**
 * Deconstructed path to a file.
 */
#[derive(Clone, Eq, PartialEq, Default, Debug, Hash)]
pub struct AssetPath {
    pub protocol: String,
    pub body: String,
    pub extension: String,
}

impl AssetPath {

    pub fn parse(path: &str, default_protocol: Option<&str>) -> Result<Self, LoadError> {
        let protocol: Option<&str>;
        let body: &str;
        let extension: &str;
        let mut remainder = path;

        // Reads protocol
        match remainder.split_once("://") {
            Some((left, right)) => {
                protocol = Some(left);
                remainder = right;
            },
            None => protocol = None,
        };
        let Some(protocol) = protocol.or(default_protocol) else {
            return Err(LoadError::NoDefaultProtocol)
        };

        // Reads body and extension
        match remainder.split_once(".") {
            Some((left, right)) => {
                body = left;
                extension = right;
            },
            None => return Err(LoadError::PathMissingExtension),
        };

        Ok(Self {
            protocol: protocol.into(),
            body: body.into(),
            extension: extension.into()
        })
    }

    /// Body and extension. No protocol.
    pub fn without_protocol(&self) -> String {
        format!("{}.{}", self.body, self.extension)
    }
}

impl fmt::Display for AssetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}.{}", self.protocol, self.body, self.extension)?;
        Ok(())
    }
}

/**
 * Wrapper for the hash of a path.
 */
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct PathHash(pub u64);
impl PathHash {
    pub fn of(path: &str) -> Self {
        Self(fxhash::hash64(path))
    }
}