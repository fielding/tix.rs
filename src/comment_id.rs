//! Comment identifiers.

use std::fmt;

/// A comment id: a 26-character ULID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommentId(String);

impl CommentId {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
