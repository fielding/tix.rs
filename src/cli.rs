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
        store::Error::DatabaseBusy | store::Error::Sqlite { .. } | store::Error::Io { .. } => 1,
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "tix",
    version,
    about = "minimal issue tracker for humans and agents"
)]
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
        #[arg(short, long, default_value_t = crate::issue::DEFAULT_PRIORITY,
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

/// A command failure, unified across the store and the parse boundary.
enum Failure {
    Store(store::Error),
    /// CLI-level parse rejection (bad status/kind names): exit 5.
    Validation(String),
    /// Planned feature whose store support has not landed yet: exit 1.
    NotImplemented(&'static str),
}

impl From<store::Error> for Failure {
    fn from(err: store::Error) -> Self {
        Failure::Store(err)
    }
}

impl Failure {
    fn code(&self) -> &'static str {
        match self {
            Failure::Store(err) => error_code(err),
            Failure::Validation(_) => "validation_failure",
            Failure::NotImplemented(_) => "not_implemented",
        }
    }

    fn exit(&self) -> u8 {
        match self {
            Failure::Store(err) => exit_code(err),
            Failure::Validation(_) => 5,
            Failure::NotImplemented(_) => 1,
        }
    }

    fn message(&self) -> String {
        match self {
            Failure::Store(err) => err.to_string(),
            Failure::Validation(message) => message.clone(),
            Failure::NotImplemented(verb) => {
                format!("{verb} is a planned feature and not implemented yet")
            }
        }
    }
}

/// Issue DTO for `--json` output; the external contract, not the domain type.
#[derive(Debug, Serialize)]
struct IssueDto {
    id: String,
    title: String,
    body: String,
    status: &'static str,
    priority: i32,
    assignee: String,
    created_at: i64,
    updated_at: i64,
    tags: Vec<String>,
}

impl IssueDto {
    fn from_issue(issue: &crate::issue::Issue) -> Self {
        Self {
            id: issue.id.to_string(),
            title: issue.title.clone(),
            body: issue.body.clone(),
            status: issue.status.as_str(),
            priority: issue.priority,
            assignee: issue.assignee.clone(),
            created_at: issue.created_at,
            updated_at: issue.updated_at,
            tags: issue.tags.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
struct CommentDto {
    id: String,
    issue_id: String,
    author: String,
    body: String,
    created_at: i64,
}

impl CommentDto {
    fn from_comment(comment: &crate::comment::Comment) -> Self {
        Self {
            id: comment.id.to_string(),
            issue_id: comment.issue_id.to_string(),
            author: comment.author.clone(),
            body: comment.body.clone(),
            created_at: comment.created_at,
        }
    }
}

fn emit_json<T: Serialize>(data: T) {
    let envelope = Envelope {
        schema_version: SCHEMA_VERSION,
        data,
    };
    println!(
        "{}",
        serde_json::to_string(&envelope).expect("envelope DTOs always serialize")
    );
}

fn print_issue_row(issue: &crate::issue::Issue) {
    println!(
        "{:<16} {:<11} {:<3} {:<12} {}",
        issue.id,
        issue.status.as_str(),
        format!("p{}", issue.priority),
        if issue.assignee.is_empty() {
            "-"
        } else {
            &issue.assignee
        },
        issue.title
    );
}

fn print_issue_table(issues: &[crate::issue::Issue]) {
    println!(
        "{:<16} {:<11} {:<3} {:<12} TITLE",
        "ID", "STATUS", "PRI", "ASSIGNEE"
    );
    for issue in issues {
        print_issue_row(issue);
    }
}

fn parse_status(input: &str) -> Result<crate::status::Status, Failure> {
    input
        .parse()
        .map_err(|err: crate::status::ParseStatusError| Failure::Validation(err.to_string()))
}

fn parse_dep_kind(input: &str) -> Result<crate::dep_kind::DepKind, Failure> {
    input
        .parse()
        .map_err(|err: crate::dep_kind::ParseDepKindError| Failure::Validation(err.to_string()))
}

fn open_store() -> Result<store::Store, Failure> {
    let cwd = std::env::current_dir().map_err(|source| store::Error::Io { source })?;
    let env = std::env::var("TIX_STORE").ok();
    let dir = store::discover(&cwd, env.as_deref())?;
    Ok(store::Store::open(&dir)?)
}

fn execute(cli: &Cli) -> ExitCode {
    match dispatch(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            if cli.json {
                let envelope = ErrorEnvelope {
                    schema_version: SCHEMA_VERSION,
                    error: ErrorBody {
                        code: failure.code().to_owned(),
                        message: failure.message(),
                    },
                };
                println!(
                    "{}",
                    serde_json::to_string(&envelope).expect("envelope DTOs always serialize")
                );
            } else {
                eprintln!("tix: {}", failure.message());
                if let Failure::Store(err) = &failure {
                    let mut source = std::error::Error::source(err);
                    while let Some(cause) = source {
                        eprintln!("  caused by: {cause}");
                        source = cause.source();
                    }
                }
            }
            ExitCode::from(failure.exit())
        }
    }
}

fn dispatch(cli: &Cli) -> Result<(), Failure> {
    match &cli.command {
        Command::Init { prefix } => {
            let store_dir = std::env::current_dir()
                .map_err(|source| store::Error::Io { source })?
                .join(".tix");
            std::fs::create_dir_all(&store_dir).map_err(|source| store::Error::Io { source })?;
            // Only the JSONL is history; the derived cache stays untracked.
            std::fs::write(store_dir.join(".gitignore"), "*.db\n*.db-wal\n*.db-shm\n")
                .map_err(|source| store::Error::Io { source })?;
            let mut store = store::Store::open(&store_dir)?;
            if let Some(prefix) = prefix {
                store.set_prefix(prefix)?;
            }
            if cli.json {
                emit_json(serde_json::json!({
                    "store": store_dir.display().to_string(),
                    "prefix": store.prefix(),
                }));
            } else {
                println!(
                    "initialized {} (prefix: {})",
                    store_dir.display(),
                    store.prefix()
                );
            }
            Ok(())
        }
        Command::Add {
            title,
            body,
            priority,
            assignee,
            tags,
            id,
            quiet: _,
        } => {
            let mut store = open_store()?;
            let tag_refs: Vec<&str> = tags.iter().map(String::as_str).collect();
            let created = store.create_issue(&crate::issue::NewIssue {
                id: id.as_deref(),
                title,
                body,
                priority: *priority,
                assignee,
                tags: &tag_refs,
            })?;
            if cli.json {
                emit_json(IssueDto::from_issue(&store.fetch_issue(created.as_str())?));
            } else {
                println!("{created}");
            }
            Ok(())
        }
        Command::List {
            status,
            assignee,
            tag,
        } => {
            let store = open_store()?;
            let status = status.as_deref().map(parse_status).transpose()?;
            let issues = store.list(&store::ListFilter {
                status,
                assignee: assignee.as_deref(),
                tag: tag.as_deref(),
            })?;
            if cli.json {
                emit_json(issues.iter().map(IssueDto::from_issue).collect::<Vec<_>>());
            } else {
                print_issue_table(&issues);
            }
            Ok(())
        }
        Command::Show { id } => {
            let store = open_store()?;
            let issue = store.fetch_issue(id)?;
            let comments = store.comments(id)?;
            if cli.json {
                emit_json(serde_json::json!({
                    "issue": IssueDto::from_issue(&issue),
                    "comments": comments.iter().map(CommentDto::from_comment).collect::<Vec<_>>(),
                }));
            } else {
                println!("ID: {}", issue.id);
                println!("Title: {}", issue.title);
                println!("Status: {}", issue.status);
                println!("Priority: {}", issue.priority);
                println!(
                    "Assignee: {}",
                    if issue.assignee.is_empty() {
                        "-"
                    } else {
                        &issue.assignee
                    }
                );
                println!("Created: {}", issue.created_at);
                println!("Updated: {}", issue.updated_at);
                if !issue.tags.is_empty() {
                    println!("Tags: {}", issue.tags.join(", "));
                }
                if !issue.body.is_empty() {
                    println!("\n{}", issue.body);
                }
                if !comments.is_empty() {
                    println!("\nComments:");
                    for comment in &comments {
                        let author = if comment.author.is_empty() {
                            "anonymous"
                        } else {
                            &comment.author
                        };
                        println!("  [{}] {}: {}", comment.created_at, author, comment.body);
                    }
                }
            }
            Ok(())
        }
        Command::Edit {
            id,
            title,
            body,
            status,
            priority,
            assignee,
            add_tags,
            rm_tags,
            quiet: _,
        } => {
            let mut store = open_store()?;
            let status = status.as_deref().map(parse_status).transpose()?;
            let add: Vec<&str> = add_tags.iter().map(String::as_str).collect();
            let rm: Vec<&str> = rm_tags.iter().map(String::as_str).collect();
            let updated = store.update_issue(
                id,
                &crate::issue::IssueUpdate {
                    title: title.as_deref(),
                    body: body.as_deref(),
                    status,
                    priority: *priority,
                    assignee: assignee.as_deref(),
                    add_tags: &add,
                    rm_tags: &rm,
                },
            )?;
            if cli.json {
                emit_json(IssueDto::from_issue(&store.fetch_issue(updated.as_str())?));
            } else {
                println!("{updated}");
            }
            Ok(())
        }
        Command::Status { id, status } => {
            let mut store = open_store()?;
            let status = parse_status(status)?;
            let updated = store.update_issue(
                id,
                &crate::issue::IssueUpdate {
                    status: Some(status),
                    ..crate::issue::IssueUpdate::default()
                },
            )?;
            if cli.json {
                emit_json(IssueDto::from_issue(&store.fetch_issue(updated.as_str())?));
            } else {
                println!("{updated}");
            }
            Ok(())
        }
        Command::Assign { id, assignee } => {
            let mut store = open_store()?;
            let updated = store.update_issue(
                id,
                &crate::issue::IssueUpdate {
                    assignee: Some(assignee),
                    ..crate::issue::IssueUpdate::default()
                },
            )?;
            if cli.json {
                emit_json(IssueDto::from_issue(&store.fetch_issue(updated.as_str())?));
            } else {
                println!("{updated}");
            }
            Ok(())
        }
        Command::Comment {
            id,
            message,
            author,
            quiet: _,
        } => {
            let mut store = open_store()?;
            let comment_id = store.add_comment(id, author, message)?;
            if cli.json {
                emit_json(serde_json::json!({ "comment_id": comment_id.to_string() }));
            } else {
                println!("{comment_id}");
            }
            Ok(())
        }
        Command::Dep { action } => {
            let mut store = open_store()?;
            match action {
                DepAction::Add { id, kind, target } => {
                    let kind = parse_dep_kind(kind)?;
                    store.add_dep(id, kind, target)?;
                }
                DepAction::Rm { id, kind, target } => {
                    let kind = parse_dep_kind(kind)?;
                    store.remove_dep(id, kind, target)?;
                }
            }
            if cli.json {
                emit_json(serde_json::json!({ "ok": true }));
            }
            Ok(())
        }
        Command::Ready { assignee } => {
            let store = open_store()?;
            let issues = store.ready(assignee.as_deref())?;
            if cli.json {
                emit_json(issues.iter().map(IssueDto::from_issue).collect::<Vec<_>>());
            } else {
                print_issue_table(&issues);
            }
            Ok(())
        }
        Command::Search { query } => {
            let store = open_store()?;
            let issues = store.search(query)?;
            if cli.json {
                emit_json(issues.iter().map(IssueDto::from_issue).collect::<Vec<_>>());
            } else {
                print_issue_table(&issues);
            }
            Ok(())
        }
        Command::Hold { .. } => Err(Failure::NotImplemented("hold")),
        Command::Unhold { .. } => Err(Failure::NotImplemented("unhold")),
        Command::Rm { .. } => Err(Failure::NotImplemented("rm")),
        Command::Export => Err(Failure::NotImplemented("export")),
        Command::Import => Err(Failure::NotImplemented("import")),
        Command::Dump => Err(Failure::NotImplemented("dump")),
    }
}
