//! Issue-id resolution (exact, prefix, ambiguity), ported from the Zig suite
//! (`src/tests.zig`, "Store: ID Resolution" and "resolveId with empty input").

mod common;

use tix::model::NewIssue;
use tix::store::Error;

// Zig: "resolveId exact match"
#[test]
fn resolve_exact_match() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Exact match",
            ..NewIssue::default()
        })
        .expect("create issue");

    let resolved = s.store.resolve_id(id.as_str()).expect("resolve id");
    assert_eq!(resolved, id);
}

// Zig: "resolveId prefix match"
#[test]
fn resolve_prefix_match() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Prefix match",
            ..NewIssue::default()
        })
        .expect("create issue");

    let prefix = &id.as_str()[..id.as_str().len().min(6)];
    let resolved = s.store.resolve_id(prefix).expect("resolve prefix");
    assert_eq!(resolved, id);
}

// Zig: "resolveId nonexistent returns IssueNotFound"
#[test]
fn resolve_nonexistent_is_not_found() {
    let s = common::open_scratch();
    let result = s.store.resolve_id("zzz-nonexistent");
    assert!(
        matches!(result, Err(Error::IssueNotFound { .. })),
        "expected IssueNotFound, got {result:?}"
    );
}

// Zig: "resolveId ambiguous returns IssueIdAmbiguous"
#[test]
fn resolve_ambiguous_prefix_errors() {
    let mut s = common::open_scratch();
    let id1 = s
        .store
        .create_issue(&NewIssue {
            title: "Issue one",
            ..NewIssue::default()
        })
        .expect("create issue");
    let _id2 = s
        .store
        .create_issue(&NewIssue {
            title: "Issue two",
            ..NewIssue::default()
        })
        .expect("create issue");

    // The shared store prefix (through the dash) matches both issues.
    let dash = id1.as_str().find('-').expect("generated id contains a dash");
    let shared = &id1.as_str()[..=dash];
    let result = s.store.resolve_id(shared);
    assert!(
        matches!(result, Err(Error::IssueIdAmbiguous { .. })),
        "expected IssueIdAmbiguous, got {result:?}"
    );
}

// Zig: "resolveId with empty string matches all (ambiguous)"
#[test]
fn resolve_empty_string_with_multiple_issues_is_ambiguous() {
    let mut s = common::open_scratch();
    for title in ["A", "B"] {
        s.store
            .create_issue(&NewIssue {
                title,
                ..NewIssue::default()
            })
            .expect("create issue");
    }

    let result = s.store.resolve_id("");
    assert!(
        matches!(result, Err(Error::IssueIdAmbiguous { .. })),
        "expected IssueIdAmbiguous, got {result:?}"
    );
}

// Zig: "resolveId with single issue and empty prefix matches"
#[test]
fn resolve_empty_string_with_single_issue_matches() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Only one",
            ..NewIssue::default()
        })
        .expect("create issue");

    let resolved = s.store.resolve_id("").expect("resolve empty prefix");
    assert_eq!(resolved, id);
}
