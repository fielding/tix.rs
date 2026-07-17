//! Edge cases: special characters, priority boundaries, status transitions,
//! long payloads, tag semantics. Ported from the Zig suite (`src/tests.zig`,
//! "Store: Special Characters" / "Store: Edge Cases" / "Priority Boundary
//! Values" / "Status Transitions" / bug-hunter buffer tests).

mod common;

use tix::model::{DepKind, IssueUpdate, NewIssue, Status};

fn create_titled(s: &mut common::Scratch, title: &str) -> String {
    s.store
        .create_issue(&NewIssue {
            title,
            ..NewIssue::default()
        })
        .expect("create issue")
        .as_str()
        .to_owned()
}

// Zig: "createIssue with special characters in title"
#[test]
fn title_with_quotes_roundtrips() {
    let mut s = common::open_scratch();
    let id = create_titled(&mut s, "Fix \"bug\" in module");
    let issue = s.store.fetch_issue(&id).expect("fetch issue");
    assert_eq!(issue.title, "Fix \"bug\" in module");
}

// Zig: "createIssue with newlines in body"
#[test]
fn body_with_newlines_roundtrips() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Newline test",
            body: "line1\nline2\nline3",
            ..NewIssue::default()
        })
        .expect("create issue");
    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.body, "line1\nline2\nline3");
}

// Zig: "createIssue with backslash in title"
#[test]
fn title_with_backslashes_roundtrips() {
    let mut s = common::open_scratch();
    let id = create_titled(&mut s, "path\\to\\file");
    let issue = s.store.fetch_issue(&id).expect("fetch issue");
    assert_eq!(issue.title, "path\\to\\file");
}

// Zig: "createIssue with tab character in body"
#[test]
fn body_with_tabs_roundtrips() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Tab test",
            body: "col1\tcol2",
            ..NewIssue::default()
        })
        .expect("create issue");
    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.body, "col1\tcol2");
}

// Zig: "addComment with special characters"
#[test]
fn comment_with_special_chars_keeps_jsonl_valid() {
    let mut s = common::open_scratch();
    let id = create_titled(&mut s, "Comment special");
    s.store
        .add_comment(id.as_str(), "alice", "This has \"quotes\" and\nnewlines")
        .expect("add comment");

    // A reimport replays the whole log; if the comment corrupted the JSONL,
    // this fails or loses the issue.
    s.store.force_reimport().expect("force reimport");
    let issue = s.store.fetch_issue(&id).expect("fetch after reimport");
    assert_eq!(issue.title, "Comment special");
    let comments = s.store.comments(&id).expect("comments after reimport");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].body, "This has \"quotes\" and\nnewlines");
}

// Zig: "createIssue with unicode in title"
#[test]
fn title_with_unicode_roundtrips() {
    let mut s = common::open_scratch();
    let id = create_titled(&mut s, "Fix café bug 🐛");
    let issue = s.store.fetch_issue(&id).expect("fetch issue");
    assert_eq!(issue.title, "Fix café bug 🐛");
}

// Zig: "createIssue with priority 1 (highest)"
#[test]
fn priority_one_roundtrips() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "P1",
            priority: 1,
            ..NewIssue::default()
        })
        .expect("create issue");
    assert_eq!(
        s.store
            .fetch_issue(id.as_str())
            .expect("fetch issue")
            .priority,
        1
    );
}

// Zig: "createIssue with priority 5 (lowest)"
#[test]
fn priority_five_roundtrips() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "P5",
            priority: 5,
            ..NewIssue::default()
        })
        .expect("create issue");
    assert_eq!(
        s.store
            .fetch_issue(id.as_str())
            .expect("fetch issue")
            .priority,
        5
    );
}

// Zig: "createIssue with priority 0"
#[test]
fn priority_zero_roundtrips() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "P0",
            priority: 0,
            ..NewIssue::default()
        })
        .expect("create issue");
    assert_eq!(
        s.store
            .fetch_issue(id.as_str())
            .expect("fetch issue")
            .priority,
        0
    );
}

// Zig: "createIssue with negative priority"
#[test]
fn negative_priority_roundtrips() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Negative pri",
            priority: -1,
            ..NewIssue::default()
        })
        .expect("create issue");
    assert_eq!(
        s.store
            .fetch_issue(id.as_str())
            .expect("fetch issue")
            .priority,
        -1
    );
}

// Zig: "createIssue with very large priority"
#[test]
fn very_large_priority_roundtrips() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Big pri",
            priority: 99999,
            ..NewIssue::default()
        })
        .expect("create issue");
    assert_eq!(
        s.store
            .fetch_issue(id.as_str())
            .expect("fetch issue")
            .priority,
        99999
    );
}

// Zig: "BUG: negative priority stored and retrieved correctly"
// (the Zig CLI crashed casting negative priorities for display; the store —
// and its JSONL roundtrip — must handle them regardless)
#[test]
fn negative_priority_survives_reimport() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Negative pri",
            priority: -5,
            ..NewIssue::default()
        })
        .expect("create issue");
    assert_eq!(
        s.store
            .fetch_issue(id.as_str())
            .expect("fetch issue")
            .priority,
        -5
    );

    s.store.force_reimport().expect("force reimport");
    assert_eq!(
        s.store
            .fetch_issue(id.as_str())
            .expect("fetch after reimport")
            .priority,
        -5
    );
}

// Zig: "createIssue with long title"
#[test]
fn long_title_roundtrips() {
    let mut s = common::open_scratch();
    let long_title = "A".repeat(500);
    let id = create_titled(&mut s, &long_title);
    let issue = s.store.fetch_issue(&id).expect("fetch issue");
    assert_eq!(issue.title, long_title);
}

// Zig: "createIssue with long body"
#[test]
fn long_body_roundtrips() {
    let mut s = common::open_scratch();
    let long_body = "B".repeat(2000);
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Long body",
            body: &long_body,
            ..NewIssue::default()
        })
        .expect("create issue");
    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.body, long_body);
}

// Zig: "updateIssue with very long body near buffer limit"
// (the Zig writer used an 8192-byte buffer; the limit is gone in Rust but the
// size class stays covered)
#[test]
fn seven_thousand_byte_body_roundtrips() {
    let mut s = common::open_scratch();
    let body = "X".repeat(7000);
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Buffer test",
            body: &body,
            ..NewIssue::default()
        })
        .expect("create issue");
    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.body.len(), 7000);
}

// Zig: "BUG: createIssue with body exceeding 8192 byte JSONL buffer should not error"
#[test]
fn body_beyond_zig_buffer_limit_roundtrips() {
    let mut s = common::open_scratch();
    let body = "X".repeat(8100);
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Overflow test",
            body: &body,
            ..NewIssue::default()
        })
        .expect("create issue");
    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.body.len(), 8100);
}

// Zig: "BUG: addComment with long comment should not error"
#[test]
fn long_comment_roundtrips() {
    let mut s = common::open_scratch();
    let id = create_titled(&mut s, "Comment overflow");
    let long_comment = "Y".repeat(4000);
    s.store
        .add_comment(&id, "author", &long_comment)
        .expect("add long comment");
    let comments = s.store.comments(&id).expect("list comments");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].body, long_comment);
}

// Zig: "BUG: appendDepJsonl buffer overflow with long IDs"
// (dep records used a 512-byte buffer in Zig; kept as a smoke check that dep
// writes with generated ids never error)
#[test]
fn dep_write_with_generated_ids_succeeds() {
    let mut s = common::open_scratch();
    let id1 = create_titled(&mut s, "Dep buf A");
    let id2 = create_titled(&mut s, "Dep buf B");
    s.store
        .add_dep(&id1, DepKind::Blocks, &id2)
        .expect("add dep");
}

// Zig: "createIssue with many tags"
#[test]
fn ten_tags_roundtrip() {
    let mut s = common::open_scratch();
    let tags: Vec<String> = (0..10).map(|i| format!("tag-{i}")).collect();
    let tag_refs: Vec<&str> = tags.iter().map(String::as_str).collect();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Many tags",
            tags: &tag_refs,
            ..NewIssue::default()
        })
        .expect("create issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    let mut got = issue.tags.clone();
    got.sort();
    let mut want = tags.clone();
    want.sort();
    assert_eq!(got, want);
}

// Zig: "duplicate tag on same issue is idempotent"
#[test]
fn duplicate_tag_collapses_to_one() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Dup tag",
            tags: &["bug", "bug"],
            ..NewIssue::default()
        })
        .expect("create issue");
    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.tags, ["bug"]);
}

// Zig: "same tag shared across issues"
#[test]
fn same_tag_shared_across_issues() {
    let mut s = common::open_scratch();
    let make = |s: &mut common::Scratch, title| {
        s.store
            .create_issue(&NewIssue {
                title,
                tags: &["shared"],
                ..NewIssue::default()
            })
            .expect("create issue")
    };
    let id1 = make(&mut s, "Shared tag 1");
    let id2 = make(&mut s, "Shared tag 2");

    let issue1 = s.store.fetch_issue(id1.as_str()).expect("fetch issue 1");
    let issue2 = s.store.fetch_issue(id2.as_str()).expect("fetch issue 2");
    assert_eq!(issue1.tags, ["shared"]);
    assert_eq!(issue2.tags, ["shared"]);
}

// Zig: "removing nonexistent tag is a no-op"
#[test]
fn removing_nonexistent_tag_is_noop() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Remove phantom tag",
            tags: &["exists"],
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                rm_tags: &["doesnotexist"],
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.tags, ["exists"]);
}

// Zig: "status transition open -> in_progress -> closed"
#[test]
fn status_walks_open_in_progress_closed() {
    let mut s = common::open_scratch();
    let id = create_titled(&mut s, "Workflow");

    assert_eq!(
        s.store.fetch_issue(&id).expect("fetch issue").status,
        Status::Open
    );

    for status in [Status::InProgress, Status::Closed] {
        s.store
            .update_issue(
                &id,
                &IssueUpdate {
                    status: Some(status),
                    ..IssueUpdate::default()
                },
            )
            .expect("update status");
        assert_eq!(
            s.store.fetch_issue(&id).expect("fetch issue").status,
            status
        );
    }
}

// Zig: "status transition closed -> open (reopen)"
#[test]
fn status_reopens_from_closed() {
    let mut s = common::open_scratch();
    let id = create_titled(&mut s, "Reopen");

    for status in [Status::Closed, Status::Open] {
        s.store
            .update_issue(
                &id,
                &IssueUpdate {
                    status: Some(status),
                    ..IssueUpdate::default()
                },
            )
            .expect("update status");
    }
    assert_eq!(
        s.store.fetch_issue(&id).expect("fetch issue").status,
        Status::Open
    );
}

// Zig: "createIssue with empty title"
#[test]
fn empty_title_is_allowed() {
    let mut s = common::open_scratch();
    let id = create_titled(&mut s, "");
    let issue = s.store.fetch_issue(&id).expect("fetch issue");
    assert_eq!(issue.title, "");
}

// Zig: "createIssue with empty assignee"
#[test]
fn empty_assignee_is_allowed() {
    let mut s = common::open_scratch();
    let id = create_titled(&mut s, "No assignee");
    let issue = s.store.fetch_issue(&id).expect("fetch issue");
    assert_eq!(issue.assignee, "");
}

// Zig: "rapid create and fetch cycle"
#[test]
fn fifty_rapid_creates_all_fetchable() {
    let mut s = common::open_scratch();
    let ids: Vec<String> = (0..50)
        .map(|i| create_titled(&mut s, &format!("Issue #{i}")))
        .collect();

    for id in &ids {
        s.store.fetch_issue(id).expect("fetch issue");
    }
}
