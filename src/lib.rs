//! tix — a minimal per-repo issue tracker for humans and agents.
//!
//! Rust rewrite of the Zig `tix` (v0.1.0). The on-disk contract is shared with
//! the Zig implementation and must remain readable in both directions:
//!
//! - A store is a directory (conventionally `.tix/`) holding `issues.jsonl`,
//!   an append-only event log. Every mutation appends a full snapshot record;
//!   nothing is ever rewritten in place.
//! - `issues.db` (plus `-wal`/`-shm`) is a derived SQLite cache with an FTS
//!   index. It is never authoritative: it self-heals from the JSONL via
//!   byte-offset incremental import, and rebuilds from scratch when the JSONL
//!   shrank (e.g. after a git merge). Deleting it loses nothing.
//! - Store discovery: the `TIX_STORE` environment variable wins, otherwise
//!   walk up from the working directory looking for a `.tix` directory.
//!
//! The CLI boundary follows the folio family conventions: a
//! `{schema_version, data|error{code,message}}` JSON envelope under `--json`,
//! data on stdout and diagnostics on stderr, and exit codes 0..=5.
//!
//! See `DECISIONS.md` for every deliberate deviation from the Zig behavior.

pub mod cli;
pub mod ids;
pub mod model;
pub mod store;
