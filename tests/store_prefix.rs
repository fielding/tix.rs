//! Store id-prefix management, ported from the Zig suite (`src/tests.zig`,
//! "Store: Prefix Management").

mod common;

use tix::model::NewIssue;
use tix::store::Store;

// Zig: "setPrefix changes prefix"
#[test]
fn set_prefix_changes_prefix() {
    let mut s = common::open_scratch();
    s.store.set_prefix("myproject").expect("set prefix");
    assert_eq!(s.store.prefix(), "myproject");
}

// Zig: "setPrefix affects new issue IDs"
#[test]
fn set_prefix_affects_new_issue_ids() {
    let mut s = common::open_scratch();
    s.store.set_prefix("proj").expect("set prefix");

    let id = s
        .store
        .create_issue(&NewIssue {
            title: "Prefixed",
            ..NewIssue::default()
        })
        .expect("create issue");
    assert!(
        id.as_str().starts_with("proj-"),
        "id {id} does not start with proj-"
    );
}

// Zig: "setPrefix persists across reopen"
#[test]
fn set_prefix_persists_across_reopen() {
    let dir = common::scratch_dir();

    {
        let mut store = Store::open(dir.path()).expect("open store");
        store.set_prefix("custom").expect("set prefix");
    }

    let store = Store::open(dir.path()).expect("reopen store");
    assert_eq!(store.prefix(), "custom");
}
