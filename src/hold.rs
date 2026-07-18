//! Holds: parking an issue without changing its status (planned feature).

use std::fmt;
use std::str::FromStr;

/// Why an issue is held. Kinds mirror the tasks-axi hold vocabulary so a tix
/// store can mirror a firstmate backlog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldKind {
    Captain,
    External,
    Load,
    Parked,
    Future,
}

impl HoldKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Captain => "captain",
            Self::External => "external",
            Self::Load => "load",
            Self::Parked => "parked",
            Self::Future => "future",
        }
    }
}

impl fmt::Display for HoldKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid hold kind {input:?} (expected captain|external|load|parked|future)")]
pub struct ParseHoldKindError {
    pub input: String,
}

impl FromStr for HoldKind {
    type Err = ParseHoldKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "captain" => Ok(Self::Captain),
            "external" => Ok(Self::External),
            "load" => Ok(Self::Load),
            "parked" => Ok(Self::Parked),
            "future" => Ok(Self::Future),
            _ => Err(ParseHoldKindError {
                input: s.to_owned(),
            }),
        }
    }
}

/// An active hold on an issue: the issue stays `open` but `ready` excludes it
/// until released (or until `until` passes, when set).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hold {
    pub reason: String,
    /// Optional expiry as milliseconds since the Unix epoch.
    pub until: Option<i64>,
    pub kind: HoldKind,
}
