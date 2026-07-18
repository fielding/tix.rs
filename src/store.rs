//! The tix store: an append-only `issues.jsonl` event log (authoritative)
//! plus a derived SQLite cache with FTS (`issues.db`, disposable).
//!
//! # On-disk contract (shared with the Zig implementation)
//!
//! `issues.jsonl` holds one JSON object per line. Every mutation appends a
//! full snapshot; the log is never rewritten. Record shapes:
//!
//! ```json
//! {"type":"issue","id":"demo-daffe9","title":"…","body":"…","status":"open",
//!  "priority":2,"assignee":"","created_at":1782864000000,
//!  "updated_at":1782864000000,"tags":["bug"]}
//! {"type":"comment","id":"01ARZ3NDEKTSV4RRFFQ69G5FAV","issue_id":"demo-daffe9",
//!  "author":"alice","body":"…","created_at":1782864000000}
//! {"type":"dep","src_id":"demo-a","dst_id":"demo-b","kind":"blocks",
//!  "state":"active"}
//! ```
//!
//! A dep record with `"state":"removed"` undoes the edge. Later records win.
//! Unknown record types and unparseable lines are skipped on import (never a
//! hard error), which is also the forward-compatibility seam for the planned
//! `hold` and `delete` record types (see `DECISIONS.md` §planned-features).
//!
//! The cache self-heals: the store remembers how many JSONL bytes it has
//! imported (`meta` table, key `jsonl_offset`). On open, new bytes are
//! imported incrementally; if the file *shrank* (e.g. a git merge rewrote
//! history) the cache is wiped and rebuilt from byte 0. Deleting
//! `issues.db*` is always safe.
#![expect(unused_variables, reason = "todo!() stubs; remove once implemented")]

mod log;

use std::path::{Path, PathBuf};

use crate::comment::Comment;
use crate::comment_id::CommentId;
use crate::dep::Dep;
use crate::dep_kind::DepKind;
use crate::hold::Hold;
use crate::issue::{Issue, IssueUpdate, NewIssue};
use crate::issue_id::IssueId;
use crate::status::Status;

/// Filters for [`Store::list`]; `None` matches everything.
#[derive(Debug, Clone, Copy, Default)]
pub struct ListFilter<'a> {
    pub status: Option<Status>,
    pub assignee: Option<&'a str>,
    pub tag: Option<&'a str>,
}

/// Store-layer failures. Callers converting these for the CLI map them onto
/// the folio-family exit codes and error envelope in [`crate::cli`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No `.tix` store directory could be discovered.
    #[error("no .tix store found")]
    StoreNotFound,
    /// `input` matched no issue id, exactly or as a unique prefix.
    #[error("issue not found: {input}")]
    IssueNotFound { input: String },
    /// `input` is a prefix of more than one issue id.
    #[error("ambiguous issue id: {input}")]
    IssueIdAmbiguous { input: String },
    /// A caller-chosen id (planned `add --id`) already exists.
    #[error("issue id already exists: {id}")]
    IssueIdExists { id: IssueId },
    /// An issue cannot block or relate to itself.
    #[error("issue cannot depend on itself")]
    SelfDependency,
    /// The SQLite cache stayed locked past the retry budget. The JSONL log is
    /// unaffected; retrying the whole operation is safe.
    #[error("store is busy")]
    DatabaseBusy,
    #[error("sqlite cache operation failed")]
    Sqlite {
        #[source]
        source: rusqlite::Error,
    },
    #[error("store I/O failed")]
    Io {
        #[source]
        source: std::io::Error,
    },
}

/// Resolve the store directory the way the Zig CLI does: an absolute or
/// cwd-relative `TIX_STORE` value wins; otherwise walk up from `start`
/// looking for a `.tix` directory. [`Error::StoreNotFound`] if neither hits.
pub fn discover(start: &Path, tix_store_env: Option<&str>) -> Result<PathBuf, Error> {
    todo!()
}

/// An open store. Constructed only through [`Store::open`].
pub struct Store {
    _p: (),
}

impl Store {
    /// Open the store rooted at `store_dir` (the `.tix` directory itself).
    ///
    /// Opens/creates the SQLite cache (`issues.db`, WAL mode, busy timeout),
    /// ensures the schema, loads the id prefix, then self-heals the cache
    /// from `issues.jsonl`:
    ///
    /// - file grew since `jsonl_offset` → import only the new bytes,
    /// - file shrank → wipe cache tables and reimport from byte 0,
    /// - unchanged → no import work.
    ///
    /// A missing `issues.jsonl` is not an error (the store is just empty).
    ///
    /// The default prefix on first open is the name of `store_dir`'s parent
    /// directory (so `/repo/.tix` → `repo`), falling back to `"do"`; it is
    /// persisted in the cache `meta` table.
    pub fn open(store_dir: &Path) -> Result<Self, Error> {
        todo!()
    }

    /// The id prefix used for generated issue ids.
    pub fn prefix(&self) -> &str {
        todo!()
    }

    /// Persist a new id prefix. Existing issue ids are unaffected.
    pub fn set_prefix(&mut self, prefix: &str) -> Result<(), Error> {
        todo!()
    }

    /// Absolute path of this store's `issues.jsonl` event log.
    pub fn jsonl_path(&self) -> &Path {
        todo!()
    }

    /// Create an issue with status `open`, stamped with the current wall
    /// clock, and append its snapshot to the JSONL log.
    ///
    /// The id is `<prefix>-<short_hash(title + now_ms)>` unless `new.id` is
    /// set (planned `add --id`), in which case an existing id yields
    /// [`Error::IssueIdExists`]. Duplicate tags in `new.tags` collapse to
    /// one.
    pub fn create_issue(&mut self, new: &NewIssue<'_>) -> Result<IssueId, Error> {
        todo!()
    }

    /// Fetch a full issue snapshot. `id_input` is resolved like
    /// [`Store::resolve_id`] (exact match, then unique prefix).
    pub fn fetch_issue(&self, id_input: &str) -> Result<Issue, Error> {
        todo!()
    }

    /// Apply a partial update, bump `updated_at` (preserving `created_at`),
    /// and append the resulting full snapshot to the JSONL log. Removing a
    /// tag the issue does not carry is a no-op. Returns the resolved id.
    pub fn update_issue(
        &mut self,
        id_input: &str,
        update: &IssueUpdate<'_>,
    ) -> Result<IssueId, Error> {
        todo!()
    }

    /// Resolve user input to a full issue id: exact match first, otherwise a
    /// prefix match that must be unique — zero matches is
    /// [`Error::IssueNotFound`], two or more is [`Error::IssueIdAmbiguous`].
    /// An empty string is a prefix of everything (so it resolves only in a
    /// single-issue store).
    pub fn resolve_id(&self, input: &str) -> Result<IssueId, Error> {
        todo!()
    }

    /// Add a comment (empty `author` = anonymous) and append it to the JSONL
    /// log. The comment id is a 26-char ULID. The issue's FTS entry is
    /// refreshed so the comment text is immediately searchable.
    pub fn add_comment(
        &mut self,
        issue_id: &str,
        author: &str,
        body: &str,
    ) -> Result<CommentId, Error> {
        todo!()
    }

    /// All comments on an issue, oldest first.
    pub fn comments(&self, issue_id: &str) -> Result<Vec<Comment>, Error> {
        todo!()
    }

    /// Record `src` −`kind`→ `dst` (e.g. src *blocks* dst) and append a
    /// `state:"active"` dep record to the JSONL log. Both endpoints must
    /// resolve; `src == dst` is [`Error::SelfDependency`]; re-adding an
    /// existing edge is idempotent.
    pub fn add_dep(&mut self, src: &str, kind: DepKind, dst: &str) -> Result<(), Error> {
        todo!()
    }

    /// Remove a dependency edge and append a `state:"removed"` dep record to
    /// the JSONL log (the log is append-only; removal is an event).
    pub fn remove_dep(&mut self, src: &str, kind: DepKind, dst: &str) -> Result<(), Error> {
        todo!()
    }

    /// Active dependency edges touching the issue (as either endpoint).
    pub fn deps(&self, issue_id: &str) -> Result<Vec<Dep>, Error> {
        todo!()
    }

    /// Issues matching the filter, most recently updated first.
    pub fn list(&self, filter: &ListFilter<'_>) -> Result<Vec<Issue>, Error> {
        todo!()
    }

    /// Issues that are ready to work: status `open`, not held (planned
    /// feature), and not transitively blocked by any open or in-progress
    /// issue via active `blocks` edges. Ordered by priority (ascending, 1
    /// first), then most recently updated. `assignee` narrows to one
    /// assignee.
    pub fn ready(&self, assignee: Option<&str>) -> Result<Vec<Issue>, Error> {
        todo!()
    }

    /// Full-text search over titles, bodies, and comment text, best match
    /// first. Matching is term-based (FTS), not substring.
    pub fn search(&self, query: &str) -> Result<Vec<Issue>, Error> {
        todo!()
    }

    /// Wipe the cache tables and rebuild everything from the JSONL log.
    /// Behaviorally a no-op for a healthy store.
    pub fn force_reimport(&mut self) -> Result<(), Error> {
        todo!()
    }

    // ─── Planned features (specced by ignored tests; not in Zig tix) ──────

    /// Place a hold on an issue (planned): appends a hold record to the JSONL
    /// log. The issue keeps its status but [`Store::ready`] excludes it while
    /// the hold is active. Holding an already-held issue replaces the hold.
    pub fn hold(&mut self, id_input: &str, hold: &Hold) -> Result<IssueId, Error> {
        todo!()
    }

    /// Release an active hold (planned): appends a hold-release record.
    /// Releasing an unheld issue is a no-op.
    pub fn unhold(&mut self, id_input: &str) -> Result<IssueId, Error> {
        todo!()
    }

    /// Delete an issue as a tombstone (planned): appends a delete record.
    /// The issue disappears from `fetch`/`list`/`ready`/`search`, but every
    /// prior record stays in the JSONL log — history is never rewritten.
    pub fn delete(&mut self, id_input: &str) -> Result<IssueId, Error> {
        todo!()
    }

    /// Export the store as JSONL event text (planned): a valid `issues.jsonl`
    /// stream that [`Store::import`] — or a fresh store adopting the file —
    /// replays to an equivalent store with identical ids.
    pub fn export(&self) -> Result<String, Error> {
        todo!()
    }

    /// Import JSONL event text into this store (planned): appends the records
    /// to this store's log and cache, preserving ids. Returns the number of
    /// records applied. Unknown record types are skipped, matching open-time
    /// import.
    pub fn import(&mut self, events_jsonl: &str) -> Result<usize, Error> {
        todo!()
    }

    /// Render the store in the tasks-axi `backlog.md` grammar (planned):
    /// `## In flight` / `## Queued` / `## Done` sections, `- [ ] <id> - <title>`
    /// bullets (`- [x]` under Done), two-space-indented body continuation
    /// lines, and trailing `(repo: …) (kind: …) (since YYYY-MM-DD)
    /// (hold: …) (hold-kind: …)` tags. See `tests/golden/backlog.md` for the
    /// pinned output and `DECISIONS.md` for the mapping rules.
    pub fn dump_backlog(&self) -> Result<String, Error> {
        todo!()
    }
}
