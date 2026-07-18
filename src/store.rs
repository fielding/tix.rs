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

mod cache;
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
    if let Some(dir) = tix_store_env {
        return Ok(PathBuf::from(dir));
    }
    start
        .ancestors()
        .map(|dir| dir.join(".tix"))
        .find(|candidate| candidate.is_dir())
        .ok_or(Error::StoreNotFound)
}

/// An open store. Constructed only through [`Store::open`].
pub struct Store {
    jsonl_path: PathBuf,
    cache: cache::Cache,
    prefix: String,
    ulid: crate::ids::UlidGenerator,
}

/// Current wall clock as milliseconds since the Unix epoch.
fn now_ms() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is set before 1970");
    i64::try_from(now.as_millis()).expect("system clock is set past year 292278994")
}

/// Map a cache failure onto the store error surface: lock contention that
/// outlived the 30s busy timeout becomes [`Error::DatabaseBusy`] (retryable),
/// everything else is a plain cache failure.
fn map_sqlite(source: rusqlite::Error) -> Error {
    if let rusqlite::Error::SqliteFailure(err, _) = &source
        && matches!(
            err.code,
            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
        )
    {
        return Error::DatabaseBusy;
    }
    Error::Sqlite { source }
}

fn map_io(source: std::io::Error) -> Error {
    Error::Io { source }
}

/// A validated snapshot becomes a domain issue. Hold state will fold in from
/// hold records when the planned feature lands; until then nothing is held.
fn snapshot_to_issue(snapshot: log::IssueSnapshot) -> Issue {
    Issue {
        id: snapshot.id,
        title: snapshot.title,
        body: snapshot.body,
        status: snapshot.status,
        priority: snapshot.priority,
        assignee: snapshot.assignee,
        created_at: snapshot.created_at,
        updated_at: snapshot.updated_at,
        tags: snapshot.tags,
        hold: None,
    }
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
        let cache = cache::Cache::open(&store_dir.join("issues.db")).map_err(map_sqlite)?;

        let prefix = match cache.meta_get("prefix").map_err(map_sqlite)? {
            Some(prefix) => prefix,
            None => {
                let derived = store_dir
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    .filter(|name| !name.is_empty())
                    .unwrap_or("do")
                    .to_owned();
                cache.meta_set("prefix", &derived).map_err(map_sqlite)?;
                derived
            }
        };

        let seed_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is set before 1970")
            .as_nanos();
        let mut store = Self {
            jsonl_path: store_dir.join("issues.jsonl"),
            cache,
            prefix,
            ulid: crate::ids::UlidGenerator::from_seed(seed_nanos as u64),
        };
        store.self_heal()?;
        Ok(store)
    }

    /// Bring the cache up to date with the log: import new bytes past the
    /// stored high-water mark, or rebuild from zero when the file shrank.
    fn self_heal(&mut self) -> Result<(), Error> {
        let size = match std::fs::metadata(&self.jsonl_path) {
            Ok(meta) => meta.len(),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(map_io(err)),
        };
        let offset = self.cache.jsonl_offset().map_err(map_sqlite)?;
        if size == offset {
            return Ok(());
        }
        if size < offset {
            self.cache.wipe().map_err(map_sqlite)?;
            self.import_from(0)?;
        } else {
            self.import_from(offset)?;
        }
        Ok(())
    }

    fn import_from(&mut self, offset: u64) -> Result<(), Error> {
        for (record, _end) in log::read_from(&self.jsonl_path, offset).map_err(map_io)? {
            self.cache.apply(record).map_err(map_sqlite)?;
        }
        let size = std::fs::metadata(&self.jsonl_path).map_err(map_io)?.len();
        self.cache.set_jsonl_offset(size).map_err(map_sqlite)
    }

    /// Append a record to the log and advance the cache high-water mark, so
    /// our own writes are not re-imported on the next open.
    fn append_record(&mut self, record: &log::Record) -> Result<(), Error> {
        log::append(&self.jsonl_path, record).map_err(map_io)?;
        let size = std::fs::metadata(&self.jsonl_path).map_err(map_io)?.len();
        self.cache.set_jsonl_offset(size).map_err(map_sqlite)
    }

    /// The id prefix used for generated issue ids.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Persist a new id prefix. Existing issue ids are unaffected.
    pub fn set_prefix(&mut self, prefix: &str) -> Result<(), Error> {
        self.cache.meta_set("prefix", prefix).map_err(map_sqlite)?;
        self.prefix = prefix.to_owned();
        Ok(())
    }

    /// Absolute path of this store's `issues.jsonl` event log.
    pub fn jsonl_path(&self) -> &Path {
        &self.jsonl_path
    }

    /// Create an issue with status `open`, stamped with the current wall
    /// clock, and append its snapshot to the JSONL log.
    ///
    /// The id is `<prefix>-<short_hash(title + now_ms)>` unless `new.id` is
    /// set (planned `add --id`), in which case an existing id yields
    /// [`Error::IssueIdExists`]. Duplicate tags in `new.tags` collapse to
    /// one.
    pub fn create_issue(&mut self, new: &NewIssue<'_>) -> Result<IssueId, Error> {
        let now = now_ms();
        let id = match new.id {
            Some(raw) => {
                let id: IssueId = raw
                    .parse()
                    .unwrap_or_else(|_| todo!("planned add --id: invalid-id error surface"));
                if self
                    .cache
                    .issue_by_id(id.as_str())
                    .map_err(map_sqlite)?
                    .is_some()
                {
                    return Err(Error::IssueIdExists { id });
                }
                id
            }
            None => format!(
                "{}-{}",
                self.prefix,
                crate::ids::short_hash(&format!("{}{}", new.title, now))
            )
            .parse()
            .expect("generated ids satisfy the id invariant (exotic prefixes are a documented compat break; see DECISIONS.md)"),
        };

        let mut tags: Vec<String> = Vec::new();
        for tag in new.tags {
            if !tags.iter().any(|t| t == tag) {
                tags.push((*tag).to_owned());
            }
        }

        let snapshot = log::IssueSnapshot {
            id: id.clone(),
            title: new.title.to_owned(),
            body: new.body.to_owned(),
            status: Status::Open,
            priority: new.priority,
            assignee: new.assignee.to_owned(),
            created_at: now,
            updated_at: now,
            tags,
        };
        self.cache.upsert_issue(&snapshot).map_err(map_sqlite)?;
        self.append_record(&log::Record::Issue(log::IssueRecord::from_snapshot(
            &snapshot,
        )))?;
        Ok(id)
    }

    /// Fetch a full issue snapshot. `id_input` is resolved like
    /// [`Store::resolve_id`] (exact match, then unique prefix).
    pub fn fetch_issue(&self, id_input: &str) -> Result<Issue, Error> {
        let id = self.resolve_id(id_input)?;
        let snapshot = self
            .cache
            .issue_by_id(id.as_str())
            .map_err(map_sqlite)?
            .ok_or_else(|| Error::IssueNotFound {
                input: id_input.to_owned(),
            })?;
        Ok(snapshot_to_issue(snapshot))
    }

    /// Apply a partial update, bump `updated_at` (preserving `created_at`),
    /// and append the resulting full snapshot to the JSONL log. Removing a
    /// tag the issue does not carry is a no-op. Returns the resolved id.
    pub fn update_issue(
        &mut self,
        id_input: &str,
        update: &IssueUpdate<'_>,
    ) -> Result<IssueId, Error> {
        let id = self.resolve_id(id_input)?;
        let current = self
            .cache
            .issue_by_id(id.as_str())
            .map_err(map_sqlite)?
            .ok_or_else(|| Error::IssueNotFound {
                input: id_input.to_owned(),
            })?;

        let mut tags = current.tags;
        for tag in update.add_tags {
            if !tags.iter().any(|t| t == tag) {
                tags.push((*tag).to_owned());
            }
        }
        tags.retain(|t| !update.rm_tags.iter().any(|rm| rm == t));

        let snapshot = log::IssueSnapshot {
            id: id.clone(),
            title: update.title.map_or(current.title, str::to_owned),
            body: update.body.map_or(current.body, str::to_owned),
            status: update.status.unwrap_or(current.status),
            priority: update.priority.unwrap_or(current.priority),
            assignee: update.assignee.map_or(current.assignee, str::to_owned),
            created_at: current.created_at,
            updated_at: now_ms(),
            tags,
        };
        self.cache.upsert_issue(&snapshot).map_err(map_sqlite)?;
        self.append_record(&log::Record::Issue(log::IssueRecord::from_snapshot(
            &snapshot,
        )))?;
        Ok(id)
    }

    /// Resolve user input to a full issue id: exact match first, otherwise a
    /// prefix match that must be unique — zero matches is
    /// [`Error::IssueNotFound`], two or more is [`Error::IssueIdAmbiguous`].
    /// An empty string is a prefix of everything (so it resolves only in a
    /// single-issue store).
    pub fn resolve_id(&self, input: &str) -> Result<IssueId, Error> {
        match self.cache.resolve(input).map_err(map_sqlite)? {
            cache::Resolved::One(id) => Ok(id),
            cache::Resolved::None => Err(Error::IssueNotFound {
                input: input.to_owned(),
            }),
            cache::Resolved::Many => Err(Error::IssueIdAmbiguous {
                input: input.to_owned(),
            }),
        }
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
        let resolved = self.resolve_id(issue_id)?;
        let now = now_ms();
        let comment = Comment {
            id: CommentId::new(self.ulid.next(now.unsigned_abs())),
            issue_id: resolved,
            author: author.to_owned(),
            body: body.to_owned(),
            created_at: now,
        };
        self.cache.insert_comment(&comment).map_err(map_sqlite)?;
        self.append_record(&log::Record::Comment(log::CommentRecord::from_comment(
            &comment,
        )))?;
        Ok(comment.id)
    }

    /// All comments on an issue, oldest first.
    pub fn comments(&self, issue_id: &str) -> Result<Vec<Comment>, Error> {
        let id = self.resolve_id(issue_id)?;
        self.cache.comments(&id).map_err(map_sqlite)
    }

    /// Record `src` −`kind`→ `dst` (e.g. src *blocks* dst) and append a
    /// `state:"active"` dep record to the JSONL log. Both endpoints must
    /// resolve; `src == dst` is [`Error::SelfDependency`]; re-adding an
    /// existing edge is idempotent.
    pub fn add_dep(&mut self, src: &str, kind: DepKind, dst: &str) -> Result<(), Error> {
        let src = self.resolve_id(src)?;
        let dst = self.resolve_id(dst)?;
        if src == dst {
            return Err(Error::SelfDependency);
        }
        self.cache
            .upsert_dep(&src, &dst, kind)
            .map_err(map_sqlite)?;
        self.append_record(&log::Record::Dep(log::DepRecord::edge(
            &src, &dst, kind, false,
        )))
    }

    /// Remove a dependency edge and append a `state:"removed"` dep record to
    /// the JSONL log (the log is append-only; removal is an event).
    pub fn remove_dep(&mut self, src: &str, kind: DepKind, dst: &str) -> Result<(), Error> {
        let src = self.resolve_id(src)?;
        let dst = self.resolve_id(dst)?;
        self.cache
            .remove_dep(&src, &dst, kind)
            .map_err(map_sqlite)?;
        self.append_record(&log::Record::Dep(log::DepRecord::edge(
            &src, &dst, kind, true,
        )))
    }

    /// Active dependency edges touching the issue (as either endpoint).
    pub fn deps(&self, issue_id: &str) -> Result<Vec<Dep>, Error> {
        let id = self.resolve_id(issue_id)?;
        self.cache.deps(&id).map_err(map_sqlite)
    }

    /// Issues matching the filter, most recently updated first.
    pub fn list(&self, filter: &ListFilter<'_>) -> Result<Vec<Issue>, Error> {
        Ok(self
            .cache
            .list(filter)
            .map_err(map_sqlite)?
            .into_iter()
            .map(snapshot_to_issue)
            .collect())
    }

    /// Issues that are ready to work: status `open`, not held (planned
    /// feature), and not transitively blocked by any open or in-progress
    /// issue via active `blocks` edges. Ordered by priority (ascending, 1
    /// first), then most recently updated. `assignee` narrows to one
    /// assignee.
    pub fn ready(&self, assignee: Option<&str>) -> Result<Vec<Issue>, Error> {
        Ok(self
            .cache
            .ready(assignee)
            .map_err(map_sqlite)?
            .into_iter()
            .map(snapshot_to_issue)
            .collect())
    }

    /// Full-text search over titles, bodies, and comment text, best match
    /// first. Matching is term-based (FTS), not substring.
    pub fn search(&self, query: &str) -> Result<Vec<Issue>, Error> {
        Ok(self
            .cache
            .search(query)
            .map_err(map_sqlite)?
            .into_iter()
            .map(snapshot_to_issue)
            .collect())
    }

    /// Wipe the cache tables and rebuild everything from the JSONL log.
    /// Behaviorally a no-op for a healthy store.
    pub fn force_reimport(&mut self) -> Result<(), Error> {
        self.cache.wipe().map_err(map_sqlite)?;
        self.import_from(0)
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
