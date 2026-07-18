//! The `issues.jsonl` wire format: one JSON record per line, append-only.
//!
//! Record structs here mirror the bytes, not the domain: raw `String`s where
//! the log may contain anything, serde defaults where the Zig writer's old
//! emissions omitted fields. Conversion into domain types happens in exactly
//! one place per record (`into_*`), and a conversion failure means "skip this
//! line" — the same self-healing tolerance the Zig reader had, and the seam
//! that lets unknown future record types pass through harmlessly.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::comment::Comment;
use crate::comment_id::CommentId;
use crate::dep_kind::DepKind;
use crate::issue_id::IssueId;
use crate::status::Status;

/// One line of `issues.jsonl`, discriminated by the `type` field.
///
/// The tag strings appear nowhere but this boundary, so the macro-generated
/// mapping is their single determinant (unlike `Status`, whose wire names
/// have four consumers and live in `as_str`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(super) enum Record {
    Issue(IssueRecord),
    Comment(CommentRecord),
    Dep(DepRecord),
}

/// A full issue snapshot. Field order matches the Zig writer byte-for-byte;
/// absence tolerance matches the Zig reader (`importIssue`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct IssueRecord {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    pub status: String,
    pub priority: i32,
    #[serde(default)]
    pub assignee: String,
    pub created_at: i64,
    /// Old lines may omit this; readers fall back to `created_at`.
    #[serde(default)]
    pub updated_at: Option<i64>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct CommentRecord {
    pub id: String,
    pub issue_id: String,
    #[serde(default)]
    pub author: String,
    pub body: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct DepRecord {
    pub src_id: String,
    pub dst_id: String,
    pub kind: String,
    #[serde(default = "default_dep_state")]
    pub state: String,
}

fn default_dep_state() -> String {
    "active".to_owned()
}

impl DepRecord {
    pub(super) fn is_removed(&self) -> bool {
        self.state == "removed"
    }
}

/// Validated view of an issue record, ready for the cache. Not a domain
/// [`crate::issue::Issue`]: hold/tombstone state folds in from other records.
pub(super) struct IssueSnapshot {
    pub id: IssueId,
    pub title: String,
    pub body: String,
    pub status: Status,
    pub priority: i32,
    pub assignee: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub tags: Vec<String>,
}

impl IssueRecord {
    /// `None` means the record is not importable (bad id or status); the
    /// caller skips the line, matching the self-healing contract.
    pub(super) fn into_snapshot(self) -> Option<IssueSnapshot> {
        Some(IssueSnapshot {
            id: self.id.parse().ok()?,
            status: self.status.parse().ok()?,
            title: self.title,
            body: self.body,
            priority: self.priority,
            assignee: self.assignee,
            created_at: self.created_at,
            updated_at: self.updated_at.unwrap_or(self.created_at),
            tags: self.tags,
        })
    }

    pub(super) fn from_snapshot(snapshot: &IssueSnapshot) -> Self {
        Self {
            id: snapshot.id.as_str().to_owned(),
            title: snapshot.title.clone(),
            body: snapshot.body.clone(),
            status: snapshot.status.as_str().to_owned(),
            priority: snapshot.priority,
            assignee: snapshot.assignee.clone(),
            created_at: snapshot.created_at,
            updated_at: Some(snapshot.updated_at),
            tags: snapshot.tags.clone(),
        }
    }
}

impl CommentRecord {
    pub(super) fn into_comment(self) -> Option<Comment> {
        Some(Comment {
            issue_id: self.issue_id.parse().ok()?,
            id: CommentId::new(self.id),
            author: self.author,
            body: self.body,
            created_at: self.created_at,
        })
    }

    pub(super) fn from_comment(comment: &Comment) -> Self {
        Self {
            id: comment.id.as_str().to_owned(),
            issue_id: comment.issue_id.as_str().to_owned(),
            author: comment.author.clone(),
            body: comment.body.clone(),
            created_at: comment.created_at,
        }
    }
}

impl DepRecord {
    /// `(src, dst, kind)`; `None` for unparseable ids/kinds — skip the line.
    pub(super) fn into_edge(self) -> Option<(IssueId, IssueId, DepKind)> {
        Some((
            self.src_id.parse().ok()?,
            self.dst_id.parse().ok()?,
            self.kind.parse().ok()?,
        ))
    }

    pub(super) fn edge(src: &IssueId, dst: &IssueId, kind: DepKind, removed: bool) -> Self {
        Self {
            src_id: src.as_str().to_owned(),
            dst_id: dst.as_str().to_owned(),
            kind: kind.as_str().to_owned(),
            state: if removed { "removed" } else { "active" }.to_owned(),
        }
    }
}

/// Append one record as one complete line. The file is opened `O_APPEND` and
/// written with a single `write_all`, so concurrent writers interleave at
/// line granularity rather than corrupting each other mid-record.
pub(super) fn append(jsonl_path: &Path, record: &Record) -> std::io::Result<()> {
    let mut line = serde_json::to_string(record).expect("record types always serialize");
    line.push('\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(jsonl_path)?;
    file.write_all(line.as_bytes())
}

/// Iterate parseable records from `offset` bytes into the file, yielding each
/// with the byte offset of the *end* of its line (the cache stores that as
/// its high-water mark). Unparseable and unknown-type lines are skipped.
///
/// A missing file is an empty log, matching the Zig reader.
pub(super) fn read_from(
    jsonl_path: &Path,
    offset: u64,
) -> std::io::Result<impl Iterator<Item = (Record, u64)>> {
    let records = match File::open(jsonl_path) {
        Ok(file) => {
            use std::io::Seek;
            let mut reader = BufReader::new(file);
            reader.seek(std::io::SeekFrom::Start(offset))?;
            let mut records = Vec::new();
            let mut pos = offset;
            let mut line = String::new();
            loop {
                line.clear();
                let n = reader.read_line(&mut line)?;
                if n == 0 {
                    break;
                }
                pos += n as u64;
                if let Ok(record) = serde_json::from_str::<Record>(line.trim_end()) {
                    records.push((record, pos));
                }
            }
            records
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(err) => return Err(err),
    };
    Ok(records.into_iter())
}

#[cfg(test)]
#[path = "log_test.rs"]
mod tests;
