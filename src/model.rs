//! Domain types shared by the store and the CLI.
//!
//! Wire names (the strings that appear in `issues.jsonl` and in CLI output)
//! are pinned here by the `as_str` methods; they are a compatibility contract
//! with existing Zig-written stores. Parsing user input into these closed
//! enums happens at the boundary — an invalid status or dep kind cannot reach
//! the store layer at all.
#![expect(unused_variables, reason = "todo!() stubs; remove once implemented")]

use std::fmt;
use std::str::FromStr;

/// Default priority for new issues (1 is highest urgency, 5 lowest; the store
/// itself accepts any `i32`, only the CLI enforces the 1..=5 range).
pub const DEFAULT_PRIORITY: i32 = 2;

/// An issue id: `<prefix>-<6-hex-hash>` when generated, or any caller-chosen
/// string for the planned `add --id`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IssueId(String);

impl IssueId {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IssueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Open,
    InProgress,
    Closed,
}

impl Status {
    /// The wire name stored in `issues.jsonl` and printed by the CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Closed => "closed",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid status {input:?} (expected open|in_progress|closed)")]
pub struct ParseStatusError {
    pub input: String,
}

impl FromStr for Status {
    type Err = ParseStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

/// Dependency edge kinds. `Blocks` participates in `ready` computation
/// (transitively); `Relates` is informational only.
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
        todo!()
    }
}

/// Why an issue is held (planned feature). Kinds mirror the tasks-axi hold
/// vocabulary so a tix store can mirror a firstmate backlog.
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
        todo!()
    }
}

/// An active hold on an issue (planned feature): the issue stays `open` but
/// `ready` excludes it until released (or until `until` passes, when set).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hold {
    pub reason: String,
    /// Optional expiry as milliseconds since the Unix epoch.
    pub until: Option<i64>,
    pub kind: HoldKind,
}

/// A full issue snapshot as fetched from the store.
///
/// Timestamps are milliseconds since the Unix epoch. Tag order is
/// unspecified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub id: IssueId,
    pub title: String,
    pub body: String,
    pub status: Status,
    pub priority: i32,
    pub assignee: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub tags: Vec<String>,
    /// Present while the issue is held (planned feature).
    pub hold: Option<Hold>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub id: CommentId,
    pub issue_id: IssueId,
    /// Empty string means anonymous, matching the Zig store.
    pub author: String,
    pub body: String,
    pub created_at: i64,
}

/// An active dependency edge: `src` `kind`s `dst` (e.g. src blocks dst).
///
/// Removed edges do not exist here — removal is an event in the JSONL log,
/// not a state on the edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dep {
    pub src: IssueId,
    pub dst: IssueId,
    pub kind: DepKind,
}

/// Input for [`crate::store::Store::create_issue`].
#[derive(Debug, Clone, Copy)]
pub struct NewIssue<'a> {
    /// Caller-chosen id (planned `add --id`); `None` generates
    /// `<prefix>-<short_hash(title + now_ms)>`.
    pub id: Option<&'a str>,
    pub title: &'a str,
    pub body: &'a str,
    pub priority: i32,
    pub assignee: &'a str,
    pub tags: &'a [&'a str],
}

impl Default for NewIssue<'_> {
    fn default() -> Self {
        Self {
            id: None,
            title: "",
            body: "",
            priority: DEFAULT_PRIORITY,
            assignee: "",
            tags: &[],
        }
    }
}

/// Partial update for [`crate::store::Store::update_issue`]; `None` fields
/// keep their current value.
#[derive(Debug, Clone, Copy, Default)]
pub struct IssueUpdate<'a> {
    pub title: Option<&'a str>,
    pub body: Option<&'a str>,
    pub status: Option<Status>,
    pub priority: Option<i32>,
    pub assignee: Option<&'a str>,
    pub add_tags: &'a [&'a str],
    pub rm_tags: &'a [&'a str],
}

/// Filters for [`crate::store::Store::list`]; `None` matches everything.
#[derive(Debug, Clone, Copy, Default)]
pub struct ListFilter<'a> {
    pub status: Option<Status>,
    pub assignee: Option<&'a str>,
    pub tag: Option<&'a str>,
}

#[cfg(test)]
#[path = "model_test.rs"]
mod tests;
