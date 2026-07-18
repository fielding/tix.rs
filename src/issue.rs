//! The issue aggregate and its store-input DTOs.

use crate::hold::Hold;
use crate::issue_id::IssueId;
use crate::status::Status;

/// Default priority for new issues (1 is highest urgency, 5 lowest; the store
/// itself accepts any `i32`, only the CLI enforces the 1..=5 range).
pub const DEFAULT_PRIORITY: i32 = 2;

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
