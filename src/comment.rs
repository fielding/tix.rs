//! Comments on issues.

use crate::comment_id::CommentId;
use crate::issue_id::IssueId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub id: CommentId,
    pub issue_id: IssueId,
    /// Empty string means anonymous, matching the Zig store.
    pub author: String,
    pub body: String,
    pub created_at: i64,
}
