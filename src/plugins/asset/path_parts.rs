use std::fmt;
use crate::LoadError;

/**
 * Deconstructed path to a file.
 */
#[derive(Clone, Eq, PartialEq, Default, Debug, Hash)]
pub struct AssetPath {
    pub protocol: String,
    pub prefix: Option<String>,
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
            prefix: None,
            body: body.into(),
            extension: extension.into()
        })
    }

    /// Body and extension. No protocol.
    pub fn without_protocol(&self) -> String {
        match self.prefix.as_deref() {
            Some(prefix) => format!("{}/{}.{}", prefix, self.body, self.extension),
            None => format!("{}.{}", self.body, self.extension),
        }
    }

    /// Parent directory of this file.
    /// None if it's at the root.
    pub fn parent(&self) -> Option<String> {
        let parts: Vec<&str> = self.body.split("/").collect();
        if parts.len() == 1 { return None }
        let parent_parts = &parts[..parts.len() - 1];
        let parent = parent_parts.join("/");
        Some(parent)
    }
}

impl fmt::Display for AssetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.prefix.as_deref() {
            Some(prefix) => write!(f, "{}://{}/{}.{}", self.protocol, prefix, self.body, self.extension),
            None => write!(f, "{}://{}.{}", self.protocol, self.body, self.extension),
        }
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