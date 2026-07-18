use super::*;

// Pins the exact byte shape the Zig writer produces and the Zig reader
// expects: compact JSON, `type` tag first, field order as declared.
#[test]
fn issue_record_wire_bytes_match_zig_writer() {
    let record = Record::Issue(IssueRecord {
        id: "demo-a3f01b".to_owned(),
        title: "T".to_owned(),
        body: "B".to_owned(),
        status: "open".to_owned(),
        priority: 2,
        assignee: String::new(),
        created_at: 1000,
        updated_at: Some(2000),
        tags: vec!["x".to_owned()],
    });
    assert_eq!(
        serde_json::to_string(&record).expect("serializes"),
        r#"{"type":"issue","id":"demo-a3f01b","title":"T","body":"B","status":"open","priority":2,"assignee":"","created_at":1000,"updated_at":2000,"tags":["x"]}"#
    );
}

// The Zig reader tolerates absent body/assignee/updated_at/tags
// (store.zig importIssue); our records must accept the same old emissions.
#[test]
fn issue_record_tolerates_zig_optional_fields() {
    let line = r#"{"type":"issue","id":"a","title":"t","status":"open","priority":1,"created_at":5}"#;
    let record: Record = serde_json::from_str(line).expect("legacy line parses");
    let Record::Issue(issue) = record else {
        panic!("expected issue record");
    };
    let snapshot = issue.into_snapshot().expect("snapshot converts");
    assert_eq!(snapshot.body, "");
    assert_eq!(snapshot.assignee, "");
    assert_eq!(snapshot.updated_at, 5, "falls back to created_at");
    assert!(snapshot.tags.is_empty());
}

// Unknown record types must parse-fail (and thus be skipped), never panic:
// this is the forward-compatibility seam hold/delete will rely on.
#[test]
fn unknown_record_type_is_a_parse_error_not_a_panic() {
    let line = r#"{"type":"hold","issue_id":"a","state":"active"}"#;
    assert!(serde_json::from_str::<Record>(line).is_err());
}

// A snapshot with a non-canonical status (the pre-repair mon store's
// doing/done) converts to None: skip the line, older snapshots stand.
#[test]
fn bad_status_snapshot_is_skipped_not_fatal() {
    let record = IssueRecord {
        id: "mon-1b6611".to_owned(),
        title: "t".to_owned(),
        body: String::new(),
        status: "done".to_owned(),
        priority: 2,
        assignee: String::new(),
        created_at: 5,
        updated_at: None,
        tags: Vec::new(),
    };
    assert!(record.into_snapshot().is_none());
}

// Dep records default absent state to active (Zig importDep).
#[test]
fn dep_record_defaults_state_to_active() {
    let line = r#"{"type":"dep","src_id":"a","dst_id":"b","kind":"blocks"}"#;
    let record: Record = serde_json::from_str(line).expect("parses");
    let Record::Dep(dep) = record else {
        panic!("expected dep record");
    };
    assert!(!dep.is_removed());
}

// Round-trip through a real file: append then read, with offsets advancing
// and garbage lines skipped.
#[test]
fn append_then_read_skips_garbage_and_tracks_offsets() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("issues.jsonl");

    let dep = Record::Dep(DepRecord::edge(
        &"a-1".parse().expect("valid id"),
        &"b-2".parse().expect("valid id"),
        crate::dep_kind::DepKind::Blocks,
        false,
    ));
    append(&path, &dep).expect("append");
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, b"{not json\n"))
        .expect("write garbage line");
    append(&path, &dep).expect("append after garbage");

    let records: Vec<(Record, u64)> = read_from(&path, 0).expect("read").collect();
    assert_eq!(records.len(), 2, "garbage line skipped");
    let file_len = std::fs::metadata(&path).expect("metadata").len();
    assert_eq!(records[1].1, file_len, "last offset is end of file");
}

// Reading a missing file is an empty log, not an error (fresh store).
#[test]
fn read_from_missing_file_is_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let count = read_from(&dir.path().join("issues.jsonl"), 0)
        .expect("read")
        .count();
    assert_eq!(count, 0);
}
