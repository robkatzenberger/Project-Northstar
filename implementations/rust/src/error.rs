use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    code: String,
    message: String,
    receipt_id: Option<String>,
    sequence: Option<i64>,
}

impl Error {
    pub fn coded(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            receipt_id: None,
            sequence: None,
        }
    }

    pub fn with_evidence(mut self, receipt_id: impl Into<String>, sequence: i64) -> Self {
        self.receipt_id = Some(receipt_id.into());
        self.sequence = Some(sequence);
        self
    }

    pub fn jcs(msg: impl Into<String>) -> Self {
        Self::coded("JCS_INVALID", msg)
    }

    pub fn hash(msg: impl Into<String>) -> Self {
        Self::coded("HASH_INVALID", msg)
    }

    pub fn claim(code: &str) -> Self {
        Self::coded(code, code)
    }

    pub fn authority(msg: impl Into<String>) -> Self {
        Self::coded("AUTHORITY_INTERNAL_ERROR", msg)
    }

    pub fn policy_compile(msg: impl Into<String>) -> Self {
        Self::coded("POLICY_COMPILE_FAILED", msg)
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn receipt_id(&self) -> Option<&str> {
        self.receipt_id.as_deref()
    }

    pub fn sequence(&self) -> Option<i64> {
        self.sequence
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
