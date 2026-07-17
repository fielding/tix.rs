//! Shared scratch-store helpers, mirroring the Zig suite's `TestCtx`: a temp
//! directory seeded with an empty `issues.jsonl`, opened as a store.
#![allow(dead_code, reason = "not every test file uses every helper")]

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use tix::store::Store;

pub struct Scratch {
    /// Held for its Drop; the directory lives as long as the store.
    pub dir: TempDir,
    pub store: Store,
}

/// Zig `TestCtx.create()`: fresh dir + empty `issues.jsonl` + open store.
pub fn open_scratch() -> Scratch {
    let dir = scratch_dir();
    let store = Store::open(dir.path()).expect("open scratch store");
    Scratch { dir, store }
}

/// A store directory seeded with an empty `issues.jsonl`, not yet opened —
/// for tests that control open/close cycles themselves.
pub fn scratch_dir() -> TempDir {
    let dir = tempfile::tempdir().expect("create scratch dir");
    fs::write(dir.path().join("issues.jsonl"), "").expect("seed empty issues.jsonl");
    dir
}

pub fn read_jsonl(store: &Store) -> String {
    fs::read_to_string(store.jsonl_path()).expect("read issues.jsonl")
}

pub fn jsonl_len(store: &Store) -> u64 {
    fs::metadata(store.jsonl_path())
        .expect("stat issues.jsonl")
        .len()
}

/// Delete the derived sqlite cache files, as the Zig reopen tests do, to
/// prove the store rebuilds itself from the JSONL alone.
pub fn delete_cache(dir: &Path) {
    for name in ["issues.db", "issues.db-wal", "issues.db-shm"] {
        let _ = fs::remove_file(dir.join(name));
    }
}
