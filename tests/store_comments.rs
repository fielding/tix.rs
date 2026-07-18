//! Comments, ported from the Zig suite (`src/tests.zig`, "Store: Comments").

mod common;

use tix::issue::NewIssue;
use tix::store::Error;

// Zig: "addComment returns ULID"
#[test]
fn add_comment_returns_ulid() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Comment test",
            ..NewIssue::default()
        })
        .expect("create issue");

    let comment_id = s
        .store
        .add_comment(id.as_str(), "alice", "Great idea")
        .expect("add comment");
    assert_eq!(comment_id.as_str().len(), 26);
}

// Zig: "addComment multiple comments on same issue"
#[test]
fn add_multiple_comments_on_same_issue() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Multi comment",
            ..NewIssue::default()
        })
        .expect("create issue");

    let c1 = s
        .store
        .add_comment(id.as_str(), "alice", "First")
        .expect("add comment");
    let c2 = s
        .store
        .add_comment(id.as_str(), "bob", "Second")
        .expect("add comment");
    let c3 = s
        .store
        .add_comment(id.as_str(), "", "Anonymous")
        .expect("add anonymous comment");

    assert_ne!(c1, c2);
    assert_ne!(c1, c3);
    assert_ne!(c2, c3);
}

// Zig: "addComment on nonexistent issue returns IssueNotFound"
#[test]
fn add_comment_on_nonexistent_issue_is_not_found() {
    let mut s = common::open_scratch();
    let result = s.store.add_comment("nonexistent-id", "alice", "hello");
    assert!(
        matches!(result, Err(Error::IssueNotFound { .. })),
        "expected IssueNotFound, got {result:?}"
    );
}

// Zig: "addComment with prefix ID resolution"
#[test]
fn add_comment_resolves_prefix_id() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Prefix comment",
            ..NewIssue::default()
        })
        .expect("create issue");

    // Comment via an id prefix; it must land on the resolved issue.
    let prefix = &id.as_str()[..id.as_str().len().min(6)];
    let comment_id = s
        .store
        .add_comment(prefix, "author", "body")
        .expect("add comment via prefix");
    assert_eq!(comment_id.as_str().len(), 26);

    let comments = s.store.comments(id.as_str()).expect("list comments");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].id, comment_id);
    assert_eq!(comments[0].issue_id, id);
}
