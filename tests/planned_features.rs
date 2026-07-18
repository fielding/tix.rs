//! Specs for agreed features the Zig tix does not have. Every test here is
//! `#[ignore]`d: they are the growth path, not part of the red baseline.
//! Un-ignore each once its feature lands. The JSONL record shapes these
//! imply (hold, delete) are specified in DECISIONS.md.

mod common;

use std::fs;

use tix::hold::{Hold, HoldKind};
use tix::issue::NewIssue;
use tix::store::{Error, Store};

// ─── add --id: caller-chosen ids ────────────────────────────────────────────

#[test]
#[ignore = "planned feature: add --id"]
fn create_with_caller_chosen_id() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            id: Some("boot-strap-1"),
            title: "Bootstrap",
            ..NewIssue::default()
        })
        .expect("create issue with chosen id");
    assert_eq!(id.as_str(), "boot-strap-1");

    let issue = s
        .store
        .fetch_issue("boot-strap-1")
        .expect("fetch by chosen id");
    assert_eq!(issue.id, id);
    assert_eq!(issue.title, "Bootstrap");
}

#[test]
#[ignore = "planned feature: add --id"]
fn create_with_colliding_id_errors_and_preserves_original() {
    let mut s = common::open_scratch();
    s.store
        .create_issue(&NewIssue {
            id: Some("dup-1"),
            title: "Original",
            ..NewIssue::default()
        })
        .expect("create original");

    let result = s.store.create_issue(&NewIssue {
        id: Some("dup-1"),
        title: "Impostor",
        ..NewIssue::default()
    });
    assert!(
        matches!(result, Err(Error::IssueIdExists { .. })),
        "expected IssueIdExists, got {result:?}"
    );

    let issue = s.store.fetch_issue("dup-1").expect("fetch original");
    assert_eq!(issue.title, "Original");
}

// ─── hold / defer ───────────────────────────────────────────────────────────

#[test]
#[ignore = "planned feature: hold"]
fn hold_records_reason_kind_and_until() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Held work",
            ..NewIssue::default()
        })
        .expect("create issue");

    let hold = Hold {
        reason: "captain decision pending".to_owned(),
        until: Some(1_790_000_000_000),
        kind: HoldKind::Captain,
    };
    s.store.hold(id.as_str(), &hold).expect("hold issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.hold, Some(hold));
}

#[test]
#[ignore = "planned feature: hold"]
fn ready_excludes_held_until_released() {
    let mut s = common::open_scratch();
    let free = s
        .store
        .create_issue(&NewIssue {
            title: "Free work",
            ..NewIssue::default()
        })
        .expect("create free issue");
    let held = s
        .store
        .create_issue(&NewIssue {
            title: "Held work",
            ..NewIssue::default()
        })
        .expect("create held issue");

    s.store
        .hold(
            held.as_str(),
            &Hold {
                reason: "waiting on upstream".to_owned(),
                until: None,
                kind: HoldKind::External,
            },
        )
        .expect("hold issue");

    let ready: Vec<_> = s
        .store
        .ready(None)
        .expect("ready")
        .into_iter()
        .map(|i| i.id)
        .collect();
    assert_eq!(ready.as_slice(), std::slice::from_ref(&free));

    s.store.unhold(held.as_str()).expect("release hold");
    let mut ready: Vec<_> = s
        .store
        .ready(None)
        .expect("ready after release")
        .into_iter()
        .map(|i| i.id)
        .collect();
    ready.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let mut want = [free, held];
    want.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    assert_eq!(ready, want);
}

#[test]
#[ignore = "planned feature: hold"]
fn hold_survives_cache_rebuild() {
    let dir = common::scratch_dir();
    let id;
    {
        let mut store = Store::open(dir.path()).expect("open store");
        id = store
            .create_issue(&NewIssue {
                title: "Held across reopen",
                ..NewIssue::default()
            })
            .expect("create issue");
        store
            .hold(
                id.as_str(),
                &Hold {
                    reason: "parked for later".to_owned(),
                    until: None,
                    kind: HoldKind::Parked,
                },
            )
            .expect("hold issue");
    }

    // The hold must live in the JSONL event log, not only in the cache.
    common::delete_cache(dir.path());

    let store = Store::open(dir.path()).expect("reopen store");
    let issue = store.fetch_issue(id.as_str()).expect("fetch after rebuild");
    let hold = issue.hold.expect("hold survives rebuild");
    assert_eq!(hold.reason, "parked for later");
    assert_eq!(hold.kind, HoldKind::Parked);
}

// ─── export / import ────────────────────────────────────────────────────────

#[test]
#[ignore = "planned feature: export/import"]
fn export_import_round_trip_preserves_ids_and_state() {
    let mut src = common::open_scratch();
    let a = src
        .store
        .create_issue(&NewIssue {
            title: "Exported A",
            body: "body A",
            priority: 1,
            assignee: "alice",
            tags: &["bug"],
            ..NewIssue::default()
        })
        .expect("create A");
    let b = src
        .store
        .create_issue(&NewIssue {
            title: "Exported B",
            ..NewIssue::default()
        })
        .expect("create B");
    src.store
        .add_dep(a.as_str(), tix::dep_kind::DepKind::Blocks, b.as_str())
        .expect("add dep");
    let comment = src
        .store
        .add_comment(a.as_str(), "alice", "ported comment")
        .expect("add comment");

    let events = src.store.export().expect("export");

    let mut dst = common::open_scratch();
    let applied = dst.store.import(&events).expect("import");
    assert!(
        applied >= 4,
        "expected at least 4 records applied, got {applied}"
    );

    let issue_a = dst.store.fetch_issue(a.as_str()).expect("A exists in dst");
    assert_eq!(issue_a.id, a);
    assert_eq!(issue_a.title, "Exported A");
    assert_eq!(issue_a.body, "body A");
    assert_eq!(issue_a.priority, 1);
    assert_eq!(issue_a.assignee, "alice");
    assert_eq!(issue_a.tags, ["bug"]);

    dst.store.fetch_issue(b.as_str()).expect("B exists in dst");

    let deps = dst.store.deps(a.as_str()).expect("deps in dst");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].dst, b);

    let comments = dst.store.comments(a.as_str()).expect("comments in dst");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].id, comment);
    assert_eq!(comments[0].body, "ported comment");
}

// ─── delete as tombstone ────────────────────────────────────────────────────

#[test]
#[ignore = "planned feature: delete tombstone"]
fn delete_hides_issue_everywhere() {
    let mut s = common::open_scratch();
    let keep = s
        .store
        .create_issue(&NewIssue {
            title: "Keep me",
            ..NewIssue::default()
        })
        .expect("create kept issue");
    let doomed = s
        .store
        .create_issue(&NewIssue {
            title: "Delete me",
            ..NewIssue::default()
        })
        .expect("create doomed issue");

    s.store.delete(doomed.as_str()).expect("delete issue");

    let result = s.store.fetch_issue(doomed.as_str());
    assert!(
        matches!(result, Err(Error::IssueNotFound { .. })),
        "deleted issue must be hidden from fetch, got {result:?}"
    );

    let listed: Vec<_> = s
        .store
        .list(&tix::store::ListFilter::default())
        .expect("list")
        .into_iter()
        .map(|i| i.id)
        .collect();
    assert_eq!(listed, [keep]);
}

#[test]
#[ignore = "planned feature: delete tombstone"]
fn delete_preserves_append_only_history() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Tombstoned",
            ..NewIssue::default()
        })
        .expect("create issue");

    let log_before = common::read_jsonl(&s.store);
    s.store.delete(id.as_str()).expect("delete issue");
    let log_after = common::read_jsonl(&s.store);

    // Append-only: the delete adds a record, rewrites nothing.
    assert!(
        log_after.starts_with(&log_before),
        "delete must append, not rewrite history"
    );
    assert!(
        log_after.contains("Tombstoned"),
        "original snapshot preserved"
    );
    assert!(
        log_after.contains("\"type\":\"delete\""),
        "tombstone record appended; log: {log_after}"
    );

    // The tombstone must also survive a cache rebuild.
    s.store.force_reimport().expect("force reimport");
    let result = s.store.fetch_issue(id.as_str());
    assert!(
        matches!(result, Err(Error::IssueNotFound { .. })),
        "tombstone must hold after reimport, got {result:?}"
    );
}

// ─── dump: tasks-axi backlog.md grammar ─────────────────────────────────────

// Fixture replayed through the store's own JSONL import path; ids and
// timestamps are fixed so the rendered backlog is byte-stable.
// 1782864000000 = 2026-07-01T00:00:00Z, 1783641600000 = 2026-07-10T00:00:00Z.
const DUMP_FIXTURE: &str = concat!(
    r#"{"type":"issue","id":"demo-aaaa01","title":"Ship the demo","body":"line one\nline two","status":"open","priority":2,"assignee":"","created_at":1782864000000,"updated_at":1782864000000,"tags":["repo:demo","kind:ship"]}"#,
    "\n",
    r#"{"type":"issue","id":"demo-bbbb02","title":"Waiting task","body":"","status":"open","priority":2,"assignee":"","created_at":1782864000000,"updated_at":1782864000000,"tags":[]}"#,
    "\n",
    r#"{"type":"hold","issue_id":"demo-bbbb02","state":"active","reason":"captain decision pending","kind":"captain","until":null,"created_at":1782864000000}"#,
    "\n",
    r#"{"type":"issue","id":"demo-cccc03","title":"In flight task","body":"","status":"in_progress","priority":2,"assignee":"","created_at":1782864000000,"updated_at":1782864000000,"tags":[]}"#,
    "\n",
    r#"{"type":"issue","id":"demo-dddd04","title":"Done task","body":"","status":"closed","priority":2,"assignee":"","created_at":1782864000000,"updated_at":1783641600000,"tags":[]}"#,
    "\n",
);

#[test]
#[ignore = "planned feature: dump to tasks-axi backlog.md"]
fn dump_backlog_matches_golden() {
    let dir = tempfile::tempdir().expect("create scratch dir");
    fs::write(dir.path().join("issues.jsonl"), DUMP_FIXTURE).expect("write fixture jsonl");

    let store = Store::open(dir.path()).expect("open store over fixture");
    let backlog = store.dump_backlog().expect("dump backlog");
    assert_eq!(backlog, include_str!("golden/backlog.md"));
}
