use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl Error {
    pub fn jcs(msg: impl Into<String>) -> Self {
        Self(format!("jcs: {}", msg.into()))
    }

    pub fn hash(msg: impl Into<String>) -> Self {
        Self(format!("hash: {}", msg.into()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
