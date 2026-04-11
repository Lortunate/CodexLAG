use std::fmt;

#[derive(Debug)]
pub struct CodexLagError(pub String);

impl CodexLagError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for CodexLagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CodexLAG error: {}", self.0)
    }
}

impl std::error::Error for CodexLagError {}

pub type Result<T> = std::result::Result<T, CodexLagError>;
