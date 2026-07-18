//! Issue create / fetch / update, ported from the Zig suite
//! (`src/tests.zig`, "Store: Basic Issue CRUD" and "Store: Update Issue").

mod common;

use tix::issue::{IssueUpdate, NewIssue};
use tix::status::Status;
use tix::store::Error;

// Zig: "createIssue returns ID with prefix"
#[test]
fn create_issue_returns_id_with_prefix() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Test issue",
            ..NewIssue::default()
        })
        .expect("create issue");
    let expected_prefix = format!("{}-", s.store.prefix());
    assert!(
        id.as_str().starts_with(&expected_prefix),
        "id {id} does not start with {expected_prefix:?}"
    );
}

// Zig: "createIssue and fetchIssue roundtrip"
#[test]
fn create_and_fetch_roundtrip() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "My Title",
            body: "My body text",
            priority: 3,
            assignee: "alice",
            ..NewIssue::default()
        })
        .expect("create issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.title, "My Title");
    assert_eq!(issue.body, "My body text");
    assert_eq!(issue.status, Status::Open);
    assert_eq!(issue.priority, 3);
    assert_eq!(issue.assignee, "alice");
    assert!(issue.created_at > 0);
    assert!(issue.updated_at > 0);
}

// Zig: "createIssue default priority is stored"
#[test]
fn create_issue_stores_defaults() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Defaults",
            ..NewIssue::default()
        })
        .expect("create issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.priority, 2);
    assert_eq!(issue.assignee, "");
    assert_eq!(issue.body, "");
}

// Zig: "createIssue with tags"
#[test]
fn create_issue_with_tags() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Tagged issue",
            priority: 1,
            tags: &["bug", "urgent"],
            ..NewIssue::default()
        })
        .expect("create issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    // Tag order is unspecified.
    let mut tags = issue.tags.clone();
    tags.sort();
    assert_eq!(tags, ["bug", "urgent"]);
}

// Zig: "createIssue with empty tags"
#[test]
fn create_issue_with_empty_tags() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "No tags",
            ..NewIssue::default()
        })
        .expect("create issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.tags, Vec::<String>::new());
}

// Zig: "createIssue multiple issues get unique IDs"
#[test]
fn create_issue_ids_are_unique() {
    let mut s = common::open_scratch();
    let mut make = |title| {
        s.store
            .create_issue(&NewIssue {
                title,
                ..NewIssue::default()
            })
            .expect("create issue")
    };
    let id1 = make("First");
    let id2 = make("Second");
    let id3 = make("Third");
    assert_ne!(id1, id2);
    assert_ne!(id1, id3);
    assert_ne!(id2, id3);
}

// Zig: "fetchIssue nonexistent returns IssueNotFound"
#[test]
fn fetch_nonexistent_is_not_found() {
    let s = common::open_scratch();
    let result = s.store.fetch_issue("nonexistent-abc123");
    assert!(
        matches!(result, Err(Error::IssueNotFound { .. })),
        "expected IssueNotFound, got {result:?}"
    );
}

// Zig: "updateIssue changes title"
#[test]
fn update_changes_title() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Original",
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                title: Some("Updated Title"),
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.title, "Updated Title");
}

// Zig: "updateIssue changes status"
#[test]
fn update_changes_status() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Status test",
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                status: Some(Status::InProgress),
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.status, Status::InProgress);
}

// Zig: "updateIssue changes priority"
#[test]
fn update_changes_priority() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Priority test",
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                priority: Some(5),
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.priority, 5);
}

// Zig: "updateIssue changes assignee"
#[test]
fn update_changes_assignee() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Assign test",
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                assignee: Some("bob"),
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.assignee, "bob");
}

// Zig: "updateIssue changes body"
#[test]
fn update_changes_body() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Body test",
            body: "old body",
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                body: Some("new body"),
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.body, "new body");
}

// Zig: "updateIssue multiple fields at once"
#[test]
fn update_multiple_fields_at_once() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Multi update",
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                title: Some("New Title"),
                body: Some("New body"),
                status: Some(Status::Closed),
                priority: Some(1),
                assignee: Some("charlie"),
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.title, "New Title");
    assert_eq!(issue.body, "New body");
    assert_eq!(issue.status, Status::Closed);
    assert_eq!(issue.priority, 1);
    assert_eq!(issue.assignee, "charlie");
}

// Zig: "updateIssue adds tags"
#[test]
fn update_adds_tags() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Tag add test",
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                add_tags: &["feature"],
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.tags, ["feature"]);
}

// Zig: "updateIssue removes tags"
#[test]
fn update_removes_tags() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Tag rm test",
            tags: &["bug", "wontfix"],
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                rm_tags: &["bug"],
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.tags, ["wontfix"]);
}

// Zig: "updateIssue add and remove tags simultaneously"
#[test]
fn update_adds_and_removes_tags_simultaneously() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Tag swap test",
            tags: &["old-tag"],
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                add_tags: &["new-tag"],
                rm_tags: &["old-tag"],
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.tags, ["new-tag"]);
}

// Zig: "updateIssue updates updated_at timestamp"
#[test]
fn update_bumps_updated_at_and_preserves_created_at() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Timestamp test",
            ..NewIssue::default()
        })
        .expect("create issue");

    let before = s.store.fetch_issue(id.as_str()).expect("fetch issue");

    std::thread::sleep(std::time::Duration::from_millis(2));

    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                title: Some("Changed"),
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");

    let after = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(after.created_at, before.created_at);
    assert!(after.updated_at >= before.updated_at);
}

// Zig: "updateIssue nonexistent returns IssueNotFound"
#[test]
fn update_nonexistent_is_not_found() {
    let mut s = common::open_scratch();
    let result = s.store.update_issue(
        "nonexistent-xyz",
        &IssueUpdate {
            title: Some("title"),
            ..IssueUpdate::default()
        },
    );
    assert!(
        matches!(result, Err(Error::IssueNotFound { .. })),
        "expected IssueNotFound, got {result:?}"
    );
}

// Zig: "updateIssue preserves unchanged fields"
#[test]
fn update_preserves_unchanged_fields() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Original title",
            body: "original body",
            priority: 3,
            assignee: "alice",
            ..NewIssue::default()
        })
        .expect("create issue");

    s.store
        .update_issue(
            id.as_str(),
            &IssueUpdate {
                status: Some(Status::Closed),
                ..IssueUpdate::default()
            },
        )
        .expect("update issue");

    let issue = s.store.fetch_issue(id.as_str()).expect("fetch issue");
    assert_eq!(issue.title, "Original title");
    assert_eq!(issue.body, "original body");
    assert_eq!(issue.status, Status::Closed);
    assert_eq!(issue.priority, 3);
    assert_eq!(issue.assignee, "alice");
}
