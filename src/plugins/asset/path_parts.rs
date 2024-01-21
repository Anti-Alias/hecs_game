use std::fmt;

use crate::LoadError;

/**
 * Deconstructed path to a file.
 */
#[derive(Clone, Eq, PartialEq, Default, Debug, Hash)]
pub struct PathParts {
    pub protocol: String,
    pub body: String,
    pub extension: String,
}

impl PathParts {

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
            return Err(LoadError::PathMissingProtocol)
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
}

impl fmt::Display for PathParts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}.{}", self.protocol, self.body, self.extension)?;
        Ok(())
    }
}