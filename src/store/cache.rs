//! The derived SQLite cache behind `issues.db`.
//!
//! Never authoritative: every row here is rebuildable from `issues.jsonl`,
//! which is why unparseable cached values are skipped rather than fatal —
//! the same tolerance the log reader applies to lines. The schema matches
//! the Zig implementation exactly (both binaries may open the same file).
//!
//! This module is the only place rusqlite appears; swapping the cache
//! backend later means replacing this file, not touching the store API.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::comment::Comment;
use crate::comment_id::CommentId;
use crate::dep::Dep;
use crate::dep_kind::DepKind;
use crate::issue_id::IssueId;

use super::ListFilter;
use super::log::{IssueSnapshot, Record};

pub(super) struct Cache {
    conn: Connection,
}

/// Outcome of resolving user input against issue ids.
pub(super) enum Resolved {
    None,
    One(IssueId),
    Many,
}

impl Cache {
    pub(super) fn open(db_path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "busy_timeout", 30000)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS issues (
               id TEXT PRIMARY KEY,
               title TEXT NOT NULL,
               body TEXT NOT NULL,
               status TEXT NOT NULL,
               priority INTEGER NOT NULL,
               assignee TEXT NOT NULL DEFAULT '',
               created_at INTEGER NOT NULL,
               updated_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS tags (
               id INTEGER PRIMARY KEY,
               name TEXT NOT NULL UNIQUE
             );
             CREATE TABLE IF NOT EXISTS issue_tags (
               issue_id TEXT NOT NULL,
               tag_id INTEGER NOT NULL,
               UNIQUE(issue_id, tag_id)
             );
             CREATE TABLE IF NOT EXISTS comments (
               id TEXT PRIMARY KEY,
               issue_id TEXT NOT NULL,
               author TEXT NOT NULL DEFAULT '',
               body TEXT NOT NULL,
               created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS deps (
               src_id TEXT NOT NULL,
               dst_id TEXT NOT NULL,
               kind TEXT NOT NULL,
               state TEXT NOT NULL DEFAULT 'active',
               PRIMARY KEY(src_id, dst_id, kind)
             );
             CREATE TABLE IF NOT EXISTS meta (
               key TEXT PRIMARY KEY,
               value TEXT NOT NULL
             );
             CREATE VIRTUAL TABLE IF NOT EXISTS issues_fts
               USING fts5(title, body, comments);
             CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status);
             CREATE INDEX IF NOT EXISTS idx_issues_assignee ON issues(assignee);
             CREATE INDEX IF NOT EXISTS idx_deps_dst ON deps(dst_id);",
        )?;
        Ok(Self { conn })
    }

    // ---- meta -------------------------------------------------------------

    pub(super) fn meta_get(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?", [key], |row| {
                row.get(0)
            })
            .optional()
    }

    pub(super) fn meta_set(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta(key, value) VALUES (?, ?)",
            [key, value],
        )?;
        Ok(())
    }

    pub(super) fn jsonl_offset(&self) -> rusqlite::Result<u64> {
        Ok(self
            .meta_get("jsonl_offset")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0))
    }

    pub(super) fn set_jsonl_offset(&self, offset: u64) -> rusqlite::Result<()> {
        self.meta_set("jsonl_offset", &offset.to_string())
    }

    // ---- import -----------------------------------------------------------

    /// Wipe all derived rows (full-reimport preamble). `meta` survives: the
    /// prefix is configuration, not derived state, and the caller resets
    /// `jsonl_offset` itself after reimporting.
    pub(super) fn wipe(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "DELETE FROM issues;
             DELETE FROM comments;
             DELETE FROM deps;
             DELETE FROM issue_tags;
             DELETE FROM issues_fts;",
        )
    }

    /// Apply one log record. Records that fail domain conversion are skipped,
    /// mirroring the log reader's line tolerance.
    pub(super) fn apply(&self, record: Record) -> rusqlite::Result<()> {
        match record {
            Record::Issue(issue) => match issue.into_snapshot() {
                Some(snapshot) => self.upsert_issue(&snapshot),
                None => Ok(()),
            },
            Record::Comment(comment) => match comment.into_comment() {
                Some(comment) => self.insert_comment(&comment),
                None => Ok(()),
            },
            Record::Dep(dep) => {
                let removed = dep.is_removed();
                match dep.into_edge() {
                    Some((src, dst, kind)) if removed => self.remove_dep(&src, &dst, kind),
                    Some((src, dst, kind)) => self.upsert_dep(&src, &dst, kind),
                    None => Ok(()),
                }
            }
        }
    }

    pub(super) fn upsert_issue(&self, snapshot: &IssueSnapshot) -> rusqlite::Result<()> {
        // ON CONFLICT UPDATE (not INSERT OR REPLACE, which the Zig version
        // uses): REPLACE deletes and re-inserts, changing the rowid and
        // orphaning the old FTS row. Keeping the rowid stable keeps the FTS
        // table garbage-free; observable search behavior is identical.
        self.conn.execute(
            "INSERT INTO issues(id, title, body, status, priority, assignee, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
               title = ?2, body = ?3, status = ?4, priority = ?5,
               assignee = ?6, created_at = ?7, updated_at = ?8",
            params![
                snapshot.id.as_str(),
                snapshot.title,
                snapshot.body,
                snapshot.status.as_str(),
                snapshot.priority,
                snapshot.assignee,
                snapshot.created_at,
                snapshot.updated_at,
            ],
        )?;

        self.conn.execute(
            "DELETE FROM issue_tags WHERE issue_id = ?",
            [snapshot.id.as_str()],
        )?;
        for tag in &snapshot.tags {
            self.add_tag(&snapshot.id, tag)?;
        }

        self.refresh_fts(&snapshot.id)
    }

    pub(super) fn add_tag(&self, issue_id: &IssueId, tag: &str) -> rusqlite::Result<()> {
        self.conn
            .execute("INSERT OR IGNORE INTO tags(name) VALUES (?)", [tag])?;
        self.conn.execute(
            "INSERT OR IGNORE INTO issue_tags(issue_id, tag_id)
             SELECT ?, id FROM tags WHERE name = ?",
            [issue_id.as_str(), tag],
        )?;
        Ok(())
    }

    pub(super) fn insert_comment(&self, comment: &Comment) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO comments(id, issue_id, author, body, created_at)
             VALUES (?, ?, ?, ?, ?)",
            params![
                comment.id.as_str(),
                comment.issue_id.as_str(),
                comment.author,
                comment.body,
                comment.created_at,
            ],
        )?;
        self.refresh_fts(&comment.issue_id)
    }

    pub(super) fn upsert_dep(&self, src: &IssueId, dst: &IssueId, kind: DepKind) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO deps(src_id, dst_id, kind, state) VALUES (?, ?, ?, 'active')",
            [src.as_str(), dst.as_str(), kind.as_str()],
        )?;
        Ok(())
    }

    pub(super) fn remove_dep(
        &self,
        src: &IssueId,
        dst: &IssueId,
        kind: DepKind,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM deps WHERE src_id = ? AND dst_id = ? AND kind = ?",
            [src.as_str(), dst.as_str(), kind.as_str()],
        )?;
        Ok(())
    }

    /// Rebuild the issue's FTS row: title + body + all comment bodies
    /// (space-joined), so comment text is searchable immediately.
    fn refresh_fts(&self, issue_id: &IssueId) -> rusqlite::Result<()> {
        let Some((rowid, title, body)) = self
            .conn
            .query_row(
                "SELECT rowid, title, body FROM issues WHERE id = ?",
                [issue_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
        else {
            return Ok(());
        };

        let mut stmt = self
            .conn
            .prepare("SELECT body FROM comments WHERE issue_id = ? ORDER BY created_at ASC")?;
        let comments = stmt
            .query_map([issue_id.as_str()], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .join(" ");

        self.conn
            .execute("DELETE FROM issues_fts WHERE rowid = ?", [rowid])?;
        self.conn.execute(
            "INSERT INTO issues_fts(rowid, title, body, comments) VALUES (?, ?, ?, ?)",
            params![rowid, title, body, comments],
        )?;
        Ok(())
    }

    // ---- queries ----------------------------------------------------------

    /// Exact-match fetch; `None` when absent (or when a cached row predates
    /// validation and no longer parses — derived state, so skipping is safe).
    pub(super) fn issue_by_id(&self, id: &str) -> rusqlite::Result<Option<IssueSnapshot>> {
        let snapshot = self
            .conn
            .query_row(
                "SELECT id, title, body, status, priority, assignee, created_at, updated_at
                 FROM issues WHERE id = ?",
                [id],
                Self::row_to_snapshot,
            )
            .optional()?
            .flatten();
        match snapshot {
            Some(mut snapshot) => {
                snapshot.tags = self.tags_of(&snapshot.id)?;
                Ok(Some(snapshot))
            }
            None => Ok(None),
        }
    }

    /// Resolve user input: exact id match wins, else unique-prefix match.
    /// LIKE metacharacters in the input are escaped — `_` and `%` are legal
    /// id characters, not wildcards (validation fixes bad data; escaping
    /// fixes good data in a dangerous context).
    pub(super) fn resolve(&self, input: &str) -> rusqlite::Result<Resolved> {
        if let Some(snapshot) = self.issue_by_id(input)? {
            return Ok(Resolved::One(snapshot.id));
        }

        let escaped = input
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("{escaped}%");
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM issues WHERE id LIKE ? ESCAPE '\\' LIMIT 2")?;
        let ids = stmt
            .query_map([&pattern], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        match ids.as_slice() {
            [] => Ok(Resolved::None),
            [one] => Ok(match one.parse() {
                Ok(id) => Resolved::One(id),
                Err(_) => Resolved::None,
            }),
            _ => Ok(Resolved::Many),
        }
    }

    pub(super) fn list(&self, filter: &ListFilter<'_>) -> rusqlite::Result<Vec<IssueSnapshot>> {
        let mut sql = String::from(
            "SELECT id, title, body, status, priority, assignee, created_at, updated_at
             FROM issues i WHERE 1=1",
        );
        let mut binds: Vec<&dyn rusqlite::ToSql> = Vec::new();
        let status_str;
        if let Some(status) = filter.status {
            status_str = status.as_str();
            sql.push_str(" AND i.status = ?");
            binds.push(&status_str);
        }
        if let Some(assignee) = &filter.assignee {
            sql.push_str(" AND i.assignee = ?");
            binds.push(assignee);
        }
        if let Some(tag) = &filter.tag {
            sql.push_str(
                " AND EXISTS (SELECT 1 FROM issue_tags it JOIN tags t ON t.id = it.tag_id
                  WHERE it.issue_id = i.id AND t.name = ?)",
            );
            binds.push(tag);
        }
        sql.push_str(" ORDER BY i.updated_at DESC");
        self.query_snapshots(&sql, binds.as_slice())
    }

    /// Open issues not transitively blocked by any open/in-progress issue
    /// through active `blocks` edges. SQL preserved from the Zig cmdReady.
    pub(super) fn ready(&self, assignee: Option<&str>) -> rusqlite::Result<Vec<IssueSnapshot>> {
        let mut sql = String::from(
            "WITH RECURSIVE blockers(src, dst) AS (
               SELECT d.src_id, d.dst_id FROM deps d
               JOIN issues si ON si.id = d.src_id
               WHERE d.kind = 'blocks' AND d.state = 'active'
                 AND si.status IN ('open','in_progress')
               UNION
               SELECT b.src, d.dst_id FROM blockers b
               JOIN deps d ON d.src_id = b.dst AND d.kind = 'blocks' AND d.state = 'active'
               JOIN issues si ON si.id = d.src_id
               WHERE si.status IN ('open','in_progress')
             )
             SELECT i.id, i.title, i.body, i.status, i.priority, i.assignee,
                    i.created_at, i.updated_at
             FROM issues i
             WHERE i.status = 'open'
               AND NOT EXISTS (
                 SELECT 1 FROM blockers b JOIN issues bi ON bi.id = b.src
                 WHERE b.dst = i.id AND bi.status IN ('open','in_progress'))",
        );
        let mut binds: Vec<&dyn rusqlite::ToSql> = Vec::new();
        if let Some(assignee) = &assignee {
            sql.push_str(" AND i.assignee = ?");
            binds.push(assignee);
        }
        sql.push_str(" ORDER BY i.priority, i.updated_at DESC");
        self.query_snapshots(&sql, binds.as_slice())
    }

    pub(super) fn search(&self, query: &str) -> rusqlite::Result<Vec<IssueSnapshot>> {
        self.query_snapshots(
            "SELECT i.id, i.title, i.body, i.status, i.priority, i.assignee,
                    i.created_at, i.updated_at
             FROM issues_fts f JOIN issues i ON i.rowid = f.rowid
             WHERE issues_fts MATCH ?
             ORDER BY bm25(issues_fts), i.updated_at DESC",
            &[&query],
        )
    }

    pub(super) fn comments(&self, issue_id: &IssueId) -> rusqlite::Result<Vec<Comment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, author, body, created_at FROM comments
             WHERE issue_id = ? ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([issue_id.as_str()], |row| {
            Ok(Comment {
                id: CommentId::new(row.get::<_, String>(0)?),
                issue_id: issue_id.clone(),
                author: row.get(1)?,
                body: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    /// Active dependency edges touching `issue_id`, in either direction.
    pub(super) fn deps(&self, issue_id: &IssueId) -> rusqlite::Result<Vec<Dep>> {
        let mut stmt = self.conn.prepare(
            "SELECT src_id, dst_id, kind FROM deps
             WHERE state = 'active' AND (src_id = ? OR dst_id = ?)
             ORDER BY src_id, dst_id, kind",
        )?;
        let rows = stmt.query_map([issue_id.as_str(), issue_id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut deps = Vec::new();
        for row in rows {
            let (src, dst, kind) = row?;
            // Zig-written caches may hold rows ours would not create; skip
            // what no longer parses rather than failing a derived read.
            let (Ok(src), Ok(dst), Ok(kind)) = (src.parse(), dst.parse(), kind.parse()) else {
                continue;
            };
            deps.push(Dep { src, dst, kind });
        }
        Ok(deps)
    }

    // ---- row plumbing -----------------------------------------------------

    fn query_snapshots(
        &self,
        sql: &str,
        binds: &[&dyn rusqlite::ToSql],
    ) -> rusqlite::Result<Vec<IssueSnapshot>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(binds, Self::row_to_snapshot)?;
        let mut snapshots = Vec::new();
        for row in rows {
            let Some(mut snapshot) = row? else { continue };
            snapshot.tags = self.tags_of(&snapshot.id)?;
            snapshots.push(snapshot);
        }
        Ok(snapshots)
    }

    /// `Ok(None)` for rows whose id/status no longer parse: the cache is
    /// derived, so skipping mirrors the log reader's line tolerance.
    fn row_to_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<Option<IssueSnapshot>> {
        let id: String = row.get(0)?;
        let status: String = row.get(3)?;
        let (Ok(id), Ok(status)) = (id.parse(), status.parse()) else {
            return Ok(None);
        };
        Ok(Some(IssueSnapshot {
            id,
            status,
            title: row.get(1)?,
            body: row.get(2)?,
            priority: row.get(4)?,
            assignee: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
            tags: Vec::new(),
        }))
    }

    fn tags_of(&self, issue_id: &IssueId) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name FROM tags t JOIN issue_tags it ON it.tag_id = t.id
             WHERE it.issue_id = ?",
        )?;
        let rows = stmt.query_map([issue_id.as_str()], |row| row.get::<_, String>(0))?;
        rows.collect()
    }
}

#[cfg(test)]
#[path = "cache_test.rs"]
mod tests;
