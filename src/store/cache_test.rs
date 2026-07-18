use super::*;
use crate::status::Status;

fn scratch_cache() -> (tempfile::TempDir, Cache) {
    let dir = tempfile::tempdir().expect("tempdir");
    let cache = Cache::open(&dir.path().join("issues.db")).expect("open cache");
    (dir, cache)
}

fn snapshot(id: &str, title: &str) -> IssueSnapshot {
    IssueSnapshot {
        id: id.parse().expect("valid id"),
        title: title.to_owned(),
        body: String::new(),
        status: Status::Open,
        priority: 2,
        assignee: String::new(),
        created_at: 1000,
        updated_at: 1000,
        tags: Vec::new(),
    }
}

// Exercises the FTS5 virtual table at open time: if the bundled SQLite
// lacks FTS5 this fails here, loudly, instead of at first search.
#[test]
fn open_creates_schema_including_fts5() {
    let (_dir, cache) = scratch_cache();
    cache
        .upsert_issue(&snapshot("t-1", "searchable words"))
        .expect("upsert");
    let hits = cache.search("searchable").expect("search");
    assert_eq!(hits.len(), 1);
}

#[test]
fn upsert_preserves_rowid_and_search_reflects_updates() {
    let (_dir, cache) = scratch_cache();
    cache
        .upsert_issue(&snapshot("t-1", "original title"))
        .expect("insert");
    cache
        .upsert_issue(&snapshot("t-1", "replacement title"))
        .expect("update");
    assert!(cache.search("original").expect("search").is_empty());
    assert_eq!(cache.search("replacement").expect("search").len(), 1);
}

#[test]
fn resolve_escapes_like_metacharacters() {
    let (_dir, cache) = scratch_cache();
    cache
        .upsert_issue(&snapshot("my_repo-1234", "underscore store"))
        .expect("upsert");
    cache
        .upsert_issue(&snapshot("myXrepo-5678", "would match a bare underscore wildcard"))
        .expect("upsert");
    // `_` must match only itself: exactly one candidate, so resolution
    // succeeds; an unescaped LIKE would see two and report ambiguity.
    let resolved = cache.resolve("my_repo").expect("resolve");
    match resolved {
        Resolved::One(id) => assert_eq!(id.as_str(), "my_repo-1234"),
        _ => panic!("expected unique resolution"),
    }
}

#[test]
fn jsonl_offset_round_trips_and_defaults_to_zero() {
    let (_dir, cache) = scratch_cache();
    assert_eq!(cache.jsonl_offset().expect("get"), 0);
    cache.set_jsonl_offset(4096).expect("set");
    assert_eq!(cache.jsonl_offset().expect("get"), 4096);
}

#[test]
fn wipe_clears_derived_rows_but_keeps_meta() {
    let (_dir, cache) = scratch_cache();
    cache
        .upsert_issue(&snapshot("t-1", "doomed"))
        .expect("upsert");
    cache.meta_set("prefix", "keepme").expect("set prefix");
    cache.wipe().expect("wipe");
    assert!(cache.issue_by_id("t-1").expect("fetch").is_none());
    assert_eq!(
        cache.meta_get("prefix").expect("get").as_deref(),
        Some("keepme")
    );
}
