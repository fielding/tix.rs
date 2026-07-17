//! Full-text search, ported from the Zig suite (`src/tests.zig`,
//! "Store: FTS Search Integration" and "Bug Hunters: FTS after comment").
//!
//! The Zig tests query the FTS table with raw SQL; the cache is an internal
//! here, so the port asserts the same behavior through `Store::search`
//! (see DECISIONS.md).

mod common;

use tix::model::NewIssue;

// Zig: "FTS search finds issue by title"
#[test]
fn search_finds_issue_by_title() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Authentication bug in login",
            ..NewIssue::default()
        })
        .expect("create issue");

    let hits = s.store.search("authentication").expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, id);
}

// Zig: "FTS search finds issue by body"
#[test]
fn search_finds_issue_by_body() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Bug report",
            body: "The frobnicator is broken",
            ..NewIssue::default()
        })
        .expect("create issue");

    let hits = s.store.search("frobnicator").expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, id);
}

// Zig: "FTS search does not match unrelated terms"
#[test]
fn search_does_not_match_unrelated_terms() {
    let mut s = common::open_scratch();
    s.store
        .create_issue(&NewIssue {
            title: "Login page fix",
            body: "Fixed the auth flow",
            ..NewIssue::default()
        })
        .expect("create issue");

    let hits = s.store.search("database").expect("search");
    assert_eq!(hits, []);
}

// Zig: "BUG: FTS search should find comment text after addComment"
// (in Zig this documents a fixed bug: addComment refreshes the FTS entry, so
// comment text is searchable without a reimport)
#[test]
fn search_finds_comment_text_without_reimport() {
    let mut s = common::open_scratch();
    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Plain title",
            body: "plain body",
            ..NewIssue::default()
        })
        .expect("create issue");
    s.store
        .add_comment(id.as_str(), "alice", "supercalifragilistic")
        .expect("add comment");

    let hits = s.store.search("supercalifragilistic").expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, id);
}
