//! The `issues.jsonl` event-log contract: persistence, self-healing reimport,
//! reopen recovery, and tolerance of malformed input. Ported from the Zig
//! suite (`src/tests.zig`, "JSONL Persistence" / "JSONL Re-import" /
//! "JSONL Roundtrip" / "Bug Hunters: JSONL import with malformed data"),
//! plus three cache-behavior tests with no Zig ancestor (marked below).

mod common;

use std::fs;
use std::fs::OpenOptions;
use std::io::Write as _;

use tix::model::{IssueUpdate, NewIssue, Status};
use tix::store::{Error, Store};

// Zig: "createIssue writes to JSONL file"
#[test]
fn create_issue_writes_snapshot_to_jsonl() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "JSONL test",
            body: "body text",
            priority: 3,
            assignee: "dev",
            ..NewIssue::default()
        })
        .expect("create issue");

    let log = common::read_jsonl(&s.store);
    assert!(log.contains("\"type\":\"issue\""), "log: {log}");
    assert!(log.contains(id.as_str()), "log: {log}");
    assert!(log.contains("JSONL test"), "log: {log}");
}

// Zig: "addComment writes to JSONL file"
#[test]
fn add_comment_writes_record_to_jsonl() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Comment JSONL",
            ..NewIssue::default()
        })
        .expect("create issue");
    s.store
        .add_comment(id.as_str(), "tester", "my comment")
        .expect("add comment");

    let log = common::read_jsonl(&s.store);
    assert!(log.contains("\"type\":\"comment\""), "log: {log}");
    assert!(log.contains("my comment"), "log: {log}");
}

// Zig: "addDep writes to JSONL file"
#[test]
fn add_dep_writes_record_to_jsonl() {
    let mut s = common::open_scratch();
    let id1 = s
        .store
        .create_issue(&NewIssue {
            title: "Dep JSONL A",
            ..NewIssue::default()
        })
        .expect("create issue");
    let id2 = s
        .store
        .create_issue(&NewIssue {
            title: "Dep JSONL B",
            ..NewIssue::default()
        })
        .expect("create issue");
    s.store
        .add_dep(id1.as_str(), tix::model::DepKind::Blocks, id2.as_str())
        .expect("add dep");

    let log = common::read_jsonl(&s.store);
    assert!(log.contains("\"type\":\"dep\""), "log: {log}");
    assert!(log.contains("\"kind\":\"blocks\""), "log: {log}");
}

// Zig: "removeDep writes removed state to JSONL"
#[test]
fn remove_dep_appends_removed_record_to_jsonl() {
    let mut s = common::open_scratch();
    let id1 = s
        .store
        .create_issue(&NewIssue {
            title: "Rm dep A",
            ..NewIssue::default()
        })
        .expect("create issue");
    let id2 = s
        .store
        .create_issue(&NewIssue {
            title: "Rm dep B",
            ..NewIssue::default()
        })
        .expect("create issue");
    s.store
        .add_dep(id1.as_str(), tix::model::DepKind::Blocks, id2.as_str())
        .expect("add dep");
    s.store
        .remove_dep(id1.as_str(), tix::model::DepKind::Blocks, id2.as_str())
        .expect("remove dep");

    let log = common::read_jsonl(&s.store);
    assert!(log.contains("\"state\":\"removed\""), "log: {log}");
}

// Zig: "updateIssue appends new line to JSONL"
#[test]
fn update_issue_appends_to_jsonl() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Update JSONL",
            ..NewIssue::default()
        })
        .expect("create issue");

    let size_before = common::jsonl_len(&s.store);
    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                title: Some("Updated JSONL"),
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");
    let size_after = common::jsonl_len(&s.store);

    assert!(
        size_after > size_before,
        "expected append: {size_before} -> {size_after}"
    );
}

// Zig: "writeJsonString escapes control characters"
#[test]
fn jsonl_escapes_control_characters() {
    let mut s = common::open_scratch();
    s.store
        .create_issue(&NewIssue {
            title: "Control chars",
            body: "before\u{01}\u{02}\u{03}after",
            ..NewIssue::default()
        })
        .expect("create issue");

    let log = common::read_jsonl(&s.store);
    // Control chars must be \u-escaped, never raw bytes.
    assert!(log.contains("\\u0001"), "log: {log}");
    assert!(!log.contains('\u{01}'), "raw control byte leaked into log");
}

// Zig: "forceReimport rebuilds SQLite from JSONL"
#[test]
fn force_reimport_rebuilds_cache_from_jsonl() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Reimport test",
            body: "body",
            priority: 1,
            assignee: "dev",
            ..NewIssue::default()
        })
        .expect("create issue");
    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                add_tags: &["test-tag"],
                ..IssueUpdate::default()
            },
        )
        .expect("tag issue");

    s.store.force_reimport().expect("force reimport");

    let issue = s
        .store
        .fetch_issue(id.as_str())
        .expect("fetch after reimport");
    assert_eq!(issue.title, "Reimport test");
    assert_eq!(issue.tags, ["test-tag"]);
}

// Zig: "forceReimport with comments"
#[test]
fn force_reimport_preserves_comments() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Comment reimport",
            ..NewIssue::default()
        })
        .expect("create issue");
    let comment_id = s
        .store
        .add_comment(id.as_str(), "alice", "test comment")
        .expect("add comment");

    s.store.force_reimport().expect("force reimport");

    let issue = s
        .store
        .fetch_issue(id.as_str())
        .expect("fetch after reimport");
    assert_eq!(issue.title, "Comment reimport");
    let comments = s
        .store
        .comments(id.as_str())
        .expect("comments after reimport");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].id, comment_id);
    assert_eq!(comments[0].body, "test comment");
}

// Zig: "forceReimport with dependencies"
#[test]
fn force_reimport_preserves_dependencies() {
    let mut s = common::open_scratch();
    let id1 = s
        .store
        .create_issue(&NewIssue {
            title: "Dep reimport A",
            ..NewIssue::default()
        })
        .expect("create issue");
    let id2 = s
        .store
        .create_issue(&NewIssue {
            title: "Dep reimport B",
            ..NewIssue::default()
        })
        .expect("create issue");
    s.store
        .add_dep(id1.as_str(), tix::model::DepKind::Blocks, id2.as_str())
        .expect("add dep");

    s.store.force_reimport().expect("force reimport");

    s.store
        .fetch_issue(id1.as_str())
        .expect("fetch A after reimport");
    s.store
        .fetch_issue(id2.as_str())
        .expect("fetch B after reimport");
    let deps = s.store.deps(id1.as_str()).expect("deps after reimport");
    assert_eq!(deps.len(), 1);
}

// Zig: "JSONL roundtrip preserves special chars after reimport"
#[test]
fn reimport_preserves_special_chars() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Title with \"quotes\"",
            body: "Body with\nnewlines\tand\ttabs",
            assignee: "user\\name",
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store.force_reimport().expect("force reimport");

    let issue = s
        .store
        .fetch_issue(id.as_str())
        .expect("fetch after reimport");
    assert_eq!(issue.title, "Title with \"quotes\"");
    assert_eq!(issue.body, "Body with\nnewlines\tand\ttabs");
    assert_eq!(issue.assignee, "user\\name");
}

// Zig: "JSONL roundtrip preserves tags after reimport"
#[test]
fn reimport_preserves_tags() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Tag roundtrip",
            tags: &["alpha", "beta", "gamma"],
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store.force_reimport().expect("force reimport");

    let issue = s
        .store
        .fetch_issue(id.as_str())
        .expect("fetch after reimport");
    let mut tags = issue.tags.clone();
    tags.sort();
    assert_eq!(tags, ["alpha", "beta", "gamma"]);
}

// Zig: "store reopen recovers issues from JSONL"
#[test]
fn reopen_without_cache_recovers_issues_from_jsonl() {
    let dir = common::scratch_dir();
    let id;
    {
        let mut store = Store::open(dir.path()).expect("open store");
        id = store
            .create_issue(&NewIssue {
                title: "Persistent",
                body: "body",
                priority: 1,
                assignee: "tester",
                ..NewIssue::default()
            })
            .expect("create issue");
    }

    common::delete_cache(dir.path());

    let store = Store::open(dir.path()).expect("reopen store");
    let issue = store.fetch_issue(id.as_str()).expect("fetch after rebuild");
    assert_eq!(issue.title, "Persistent");
    assert_eq!(issue.body, "body");
    assert_eq!(issue.priority, 1);
    assert_eq!(issue.assignee, "tester");
    assert_eq!(issue.status, Status::Open);
}

// Zig: "store reopen recovers updated issue state"
#[test]
fn reopen_without_cache_recovers_updated_state() {
    let dir = common::scratch_dir();
    let id;
    {
        let mut store = Store::open(dir.path()).expect("open store");
        id = store
            .create_issue(&NewIssue {
                title: "Update reopen",
                ..NewIssue::default()
            })
            .expect("create issue");
        store
            .update_issue(
                id.as_str(),
                &IssueUpdate {
                    status: Some(Status::Closed),
                    priority: Some(5),
                    assignee: Some("bob"),
                    ..IssueUpdate::default()
                },
            )
            .expect("update issue");
    }

    common::delete_cache(dir.path());

    let store = Store::open(dir.path()).expect("reopen store");
    let issue = store.fetch_issue(id.as_str()).expect("fetch after rebuild");
    assert_eq!(issue.status, Status::Closed);
    assert_eq!(issue.priority, 5);
    assert_eq!(issue.assignee, "bob");
}

// Zig: "store opens with empty JSONL file"
#[test]
fn opens_with_empty_jsonl() {
    let _scratch = common::open_scratch();
}

// Zig: "store handles JSONL with trailing newlines"
#[test]
fn opens_with_only_blank_lines() {
    let dir = tempfile::tempdir().expect("create scratch dir");
    fs::write(dir.path().join("issues.jsonl"), "\n\n\n").expect("write jsonl");
    Store::open(dir.path()).expect("open store over blank lines");
}

// Zig: "store handles JSONL with malformed JSON lines"
#[test]
fn opens_with_malformed_json_lines() {
    let dir = tempfile::tempdir().expect("create scratch dir");
    fs::write(
        dir.path().join("issues.jsonl"),
        "not json at all\n{broken\n",
    )
    .expect("write jsonl");
    Store::open(dir.path()).expect("open store over malformed lines");
}

// Zig: "store handles JSONL with unknown record type"
#[test]
fn opens_with_unknown_record_type() {
    let dir = tempfile::tempdir().expect("create scratch dir");
    fs::write(
        dir.path().join("issues.jsonl"),
        "{\"type\":\"unknown\",\"data\":\"foo\"}\n",
    )
    .expect("write jsonl");
    Store::open(dir.path()).expect("open store over unknown record type");
}

// Zig: "store handles JSONL with missing required fields"
#[test]
fn skips_records_missing_required_fields() {
    let dir = tempfile::tempdir().expect("create scratch dir");
    // Issue record with no title: skipped on import, not a hard error.
    fs::write(
        dir.path().join("issues.jsonl"),
        "{\"type\":\"issue\",\"id\":\"test-123\",\"status\":\"open\",\"priority\":2,\"created_at\":1000}\n",
    )
    .expect("write jsonl");

    let store = Store::open(dir.path()).expect("open store");
    let result = store.resolve_id("test-123");
    assert!(
        matches!(result, Err(Error::IssueNotFound { .. })),
        "expected IssueNotFound, got {result:?}"
    );
}

// No Zig ancestor — cache-behavior spec from the rewrite brief: the store
// imports externally appended JSONL bytes incrementally (byte-offset
// tracking), so a git pull or another writer is picked up on reopen.
#[test]
fn reopen_imports_externally_appended_records() {
    let dir = common::scratch_dir();
    let id;
    {
        let mut store = Store::open(dir.path()).expect("open store");
        id = store
            .create_issue(&NewIssue {
                title: "Local issue",
                ..NewIssue::default()
            })
            .expect("create issue");
    }

    let mut file = OpenOptions::new()
        .append(true)
        .open(dir.path().join("issues.jsonl"))
        .expect("open jsonl for append");
    writeln!(
        file,
        "{{\"type\":\"issue\",\"id\":\"ext-aaaaaa\",\"title\":\"External issue\",\"body\":\"\",\"status\":\"open\",\"priority\":2,\"assignee\":\"\",\"created_at\":1000,\"updated_at\":1000,\"tags\":[]}}"
    )
    .expect("append external record");
    drop(file);

    let store = Store::open(dir.path()).expect("reopen store");
    store
        .fetch_issue(id.as_str())
        .expect("local issue survives");
    let external = store
        .fetch_issue("ext-aaaaaa")
        .expect("external issue imported");
    assert_eq!(external.title, "External issue");
}

// No Zig ancestor — cache-behavior spec from the rewrite brief: when the
// JSONL shrank (e.g. a git merge rewrote history), the cache must be wiped
// and rebuilt from byte 0 rather than trusting its stored offset.
#[test]
fn reopen_after_jsonl_truncation_fully_reimports() {
    let dir = common::scratch_dir();
    let (id1, id2);
    {
        let mut store = Store::open(dir.path()).expect("open store");
        id1 = store
            .create_issue(&NewIssue {
                title: "Kept issue",
                ..NewIssue::default()
            })
            .expect("create issue");
        id2 = store
            .create_issue(&NewIssue {
                title: "Dropped issue",
                ..NewIssue::default()
            })
            .expect("create issue");
    }

    let jsonl_path = dir.path().join("issues.jsonl");
    let log = fs::read_to_string(&jsonl_path).expect("read jsonl");
    let first_line = log.lines().next().expect("log has at least one line");
    fs::write(&jsonl_path, format!("{first_line}\n")).expect("truncate jsonl");

    let store = Store::open(dir.path()).expect("reopen store");
    store
        .fetch_issue(id1.as_str())
        .expect("first issue survives");
    let result = store.fetch_issue(id2.as_str());
    assert!(
        matches!(result, Err(Error::IssueNotFound { .. })),
        "truncated-away issue must be gone from the cache, got {result:?}"
    );
}

// No Zig ancestor — cache-behavior spec from the rewrite brief: two handles
// on one store must not fail with a busy error under interleaved writes
// (busy_timeout / bounded retry inside the store).
#[test]
fn concurrent_handles_retry_through_busy() {
    let dir = common::scratch_dir();
    let mut a = Store::open(dir.path()).expect("open first handle");
    let mut b = Store::open(dir.path()).expect("open second handle");

    let id_a = a
        .create_issue(&NewIssue {
            title: "From handle A",
            ..NewIssue::default()
        })
        .expect("create via first handle");
    let id_b = b
        .create_issue(&NewIssue {
            title: "From handle B",
            ..NewIssue::default()
        })
        .expect("create via second handle");
    assert_ne!(id_a, id_b);
}
