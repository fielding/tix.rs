//! Dependency edge kinds.

use std::fmt;
use std::str::FromStr;

/// `Blocks` participates in `ready` computation (transitively); `Relates` is
/// informational only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepKind {
    Blocks,
    Relates,
}

impl DepKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocks => "blocks",
            Self::Relates => "relates",
        }
    }
}

impl fmt::Display for DepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid dep kind {input:?} (expected blocks|relates)")]
pub struct ParseDepKindError {
    pub input: String,
}

impl FromStr for DepKind {
    type Err = ParseDepKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blocks" => Ok(Self::Blocks),
            "relates" => Ok(Self::Relates),
            _ => Err(ParseDepKindError {
                input: s.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
#[path = "dep_kind_test.rs"]
mod tests;
