//! Dependency edges, ported from the Zig suite (`src/tests.zig`,
//! "Store: Dependencies").
//!
//! The Zig test "addDep invalid kind returns InvalidDepKind" lives at the
//! parse boundary in the Rust port (`src/model_test.rs`): `DepKind` is a
//! closed enum, so an invalid kind cannot reach `add_dep`.

mod common;

use tix::dep_kind::DepKind;
use tix::issue::NewIssue;
use tix::store::Error;

fn two_issues(s: &mut common::Scratch, a: &str, b: &str) -> (String, String) {
    let id1 = s
        .store
        .create_issue(&NewIssue {
            title: a,
            ..NewIssue::default()
        })
        .expect("create first issue");
    let id2 = s
        .store
        .create_issue(&NewIssue {
            title: b,
            ..NewIssue::default()
        })
        .expect("create second issue");
    (id1.as_str().to_owned(), id2.as_str().to_owned())
}

// Zig: "addDep blocks"
#[test]
fn add_dep_blocks() {
    let mut s = common::open_scratch();
    let (id1, id2) = two_issues(&mut s, "Blocker", "Blocked");
    s.store
        .add_dep(&id1, DepKind::Blocks, &id2)
        .expect("add blocks dep");

    let deps = s.store.deps(&id1).expect("list deps");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].src.as_str(), id1);
    assert_eq!(deps[0].dst.as_str(), id2);
    assert_eq!(deps[0].kind, DepKind::Blocks);
}

// Zig: "addDep relates"
#[test]
fn add_dep_relates() {
    let mut s = common::open_scratch();
    let (id1, id2) = two_issues(&mut s, "Related A", "Related B");
    s.store
        .add_dep(&id1, DepKind::Relates, &id2)
        .expect("add relates dep");

    let deps = s.store.deps(&id1).expect("list deps");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].kind, DepKind::Relates);
}

// Zig: "addDep self-dependency returns SelfDependency"
#[test]
fn add_dep_to_self_errors() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Self dep",
            ..NewIssue::default()
        })
        .expect("create issue");

    let result = s.store.add_dep(id.as_str(), DepKind::Blocks, id.as_str());
    assert!(
        matches!(result, Err(Error::SelfDependency)),
        "expected SelfDependency, got {result:?}"
    );
}

// Zig: "addDep nonexistent source returns IssueNotFound"
#[test]
fn add_dep_nonexistent_source_is_not_found() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Real",
            ..NewIssue::default()
        })
        .expect("create issue");

    let result = s
        .store
        .add_dep("nonexistent-xyz", DepKind::Blocks, id.as_str());
    assert!(
        matches!(result, Err(Error::IssueNotFound { .. })),
        "expected IssueNotFound, got {result:?}"
    );
}

// Zig: "addDep nonexistent target returns IssueNotFound"
#[test]
fn add_dep_nonexistent_target_is_not_found() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Real",
            ..NewIssue::default()
        })
        .expect("create issue");

    let result = s
        .store
        .add_dep(id.as_str(), DepKind::Blocks, "nonexistent-xyz");
    assert!(
        matches!(result, Err(Error::IssueNotFound { .. })),
        "expected IssueNotFound, got {result:?}"
    );
}

// Zig: "removeDep"
#[test]
fn remove_dep() {
    let mut s = common::open_scratch();
    let (id1, id2) = two_issues(&mut s, "Blocker", "Blocked");
    s.store
        .add_dep(&id1, DepKind::Blocks, &id2)
        .expect("add dep");
    s.store
        .remove_dep(&id1, DepKind::Blocks, &id2)
        .expect("remove dep");

    let deps = s.store.deps(&id1).expect("list deps");
    assert_eq!(deps, []);
}

// Zig: "addDep duplicate is idempotent"
#[test]
fn add_dep_duplicate_is_idempotent() {
    let mut s = common::open_scratch();
    let (id1, id2) = two_issues(&mut s, "A", "B");
    s.store
        .add_dep(&id1, DepKind::Blocks, &id2)
        .expect("add dep");
    s.store
        .add_dep(&id1, DepKind::Blocks, &id2)
        .expect("re-add same dep");

    let deps = s.store.deps(&id1).expect("list deps");
    assert_eq!(deps.len(), 1);
}
