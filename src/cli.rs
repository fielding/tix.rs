//! The `tix` command-line boundary.
//!
//! Follows the folio family contract:
//!
//! - data on stdout, diagnostics on stderr;
//! - `--json` wraps results in `{"schema_version":1,"data":{…}}` and errors
//!   in `{"schema_version":1,"error":{"code":"…","message":"…"}}`;
//! - exit codes: 0 success, 1 operational failure, 2 usage error (clap owns
//!   this one), 3 not found, 4 ambiguity or semantic conflict, 5 validation
//!   failure.
//!
//! Store discovery: `TIX_STORE` env var, else walk up from the working
//! directory for a `.tix` directory ([`crate::store::discover`]).

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::Serialize;

use crate::store;

/// Version of the JSON envelope contract, not of the binary.
pub const SCHEMA_VERSION: u32 = 1;

/// Success envelope: `{"schema_version":1,"data":…}` on stdout.
#[derive(Debug, Serialize)]
pub struct Envelope<T: Serialize> {
    pub schema_version: u32,
    pub data: T,
}

/// Error envelope: `{"schema_version":1,"error":{…}}`.
#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    pub schema_version: u32,
    pub error: ErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    /// Stable snake_case identifier, e.g. `issue_not_found`.
    pub code: String,
    pub message: String,
}

/// Stable error code for a store failure (the `error.code` envelope field).
pub fn error_code(err: &store::Error) -> &'static str {
    match err {
        store::Error::StoreNotFound => "store_not_found",
        store::Error::IssueNotFound { .. } => "issue_not_found",
        store::Error::IssueIdAmbiguous { .. } => "issue_id_ambiguous",
        store::Error::IssueIdExists { .. } => "issue_id_exists",
        store::Error::SelfDependency => "self_dependency",
        store::Error::DatabaseBusy => "store_busy",
        store::Error::Sqlite { .. } => "cache_failure",
        store::Error::Io { .. } => "io_failure",
    }
}

/// Process exit code for a store failure, per the folio contract.
pub fn exit_code(err: &store::Error) -> u8 {
    match err {
        store::Error::StoreNotFound | store::Error::IssueNotFound { .. } => 3,
        store::Error::IssueIdAmbiguous { .. }
        | store::Error::IssueIdExists { .. }
        | store::Error::SelfDependency => 4,
        store::Error::DatabaseBusy
        | store::Error::Sqlite { .. }
        | store::Error::Io { .. } => 1,
    }
}

#[derive(Debug, Parser)]
#[command(name = "tix", version, about = "minimal issue tracker for humans and agents")]
pub struct Cli {
    /// Emit machine-readable JSON envelopes instead of human output.
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

/// One variant per verb. Free-form values (status names, dep kinds, priority)
/// stay `String`/`i32` here and are parsed into the closed `model` enums at
/// the start of execution; a parse failure is a validation error (exit 5).
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a `.tix` store in the current directory (idempotent). Also
    /// writes `.tix/.gitignore` ignoring `*.db*` so only the JSONL is
    /// tracked.
    Init {
        #[arg(long)]
        prefix: Option<String>,
    },
    /// Create an issue; prints its id (or the full issue under --json).
    #[command(visible_alias = "new")]
    Add {
        title: String,
        #[arg(short, long, default_value = "")]
        body: String,
        /// 1 (highest) to 5 (lowest).
        #[arg(short, long, default_value_t = crate::model::DEFAULT_PRIORITY,
              value_parser = clap::value_parser!(i32).range(1..=5))]
        priority: i32,
        #[arg(short, long, default_value = "")]
        assignee: String,
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
        /// Caller-chosen id (planned feature); collision is an error.
        #[arg(long)]
        id: Option<String>,
        /// Print only the id.
        #[arg(short, long)]
        quiet: bool,
    },
    /// List issues, most recently updated first.
    #[command(visible_alias = "ls")]
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long)]
        tag: Option<String>,
    },
    /// Show one issue with its comments.
    Show { id: String },
    /// Update issue fields.
    Edit {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long, value_parser = clap::value_parser!(i32).range(1..=5))]
        priority: Option<i32>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long = "add-tag")]
        add_tags: Vec<String>,
        #[arg(long = "rm-tag")]
        rm_tags: Vec<String>,
        #[arg(short, long)]
        quiet: bool,
    },
    /// Change status: open, in_progress, or closed.
    Status { id: String, status: String },
    /// Assign an issue.
    Assign { id: String, assignee: String },
    /// Add a comment.
    Comment {
        id: String,
        #[arg(short, long)]
        message: String,
        #[arg(long, default_value = "")]
        author: String,
        #[arg(short, long)]
        quiet: bool,
    },
    /// Manage dependency edges.
    Dep {
        #[command(subcommand)]
        action: DepAction,
    },
    /// List unblocked, unheld open issues, highest priority first.
    Ready {
        #[arg(long)]
        assignee: Option<String>,
    },
    /// Full-text search over titles, bodies, and comments.
    Search { query: String },
    /// Place a hold on an issue (planned feature).
    Hold {
        id: String,
        #[arg(long)]
        reason: String,
        /// Kind label: captain, external, load, parked, or future.
        #[arg(long, default_value = "captain")]
        kind: String,
        /// Optional expiry date, YYYY-MM-DD.
        #[arg(long)]
        until: Option<String>,
    },
    /// Release a hold (planned feature).
    Unhold { id: String },
    /// Tombstone an issue: hidden everywhere, history preserved (planned
    /// feature).
    Rm { id: String },
    /// Export the store as JSONL events on stdout (planned feature).
    Export,
    /// Import JSONL events from stdin (planned feature).
    Import,
    /// Render the store in the tasks-axi backlog.md grammar (planned
    /// feature).
    Dump,
}

#[derive(Debug, Subcommand)]
pub enum DepAction {
    /// `tix dep add <id> <blocks|relates> <target>`
    Add {
        id: String,
        kind: String,
        target: String,
    },
    /// `tix dep rm <id> <blocks|relates> <target>`
    Rm {
        id: String,
        kind: String,
        target: String,
    },
}

/// Parse the process arguments and run. Usage errors exit 2 via clap; store
/// errors map through [`exit_code`], with the cause chain reported on stderr
/// (or as a JSON error envelope under `--json`).
pub fn run() -> ExitCode {
    let cli = Cli::parse();
    execute(&cli)
}

#[expect(unused_variables, reason = "todo!() stub; remove once implemented")]
fn execute(cli: &Cli) -> ExitCode {
    todo!()
}
