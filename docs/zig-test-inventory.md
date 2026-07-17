# Zig test inventory → Rust port map

Every `test` block in the Zig reference (`tix/src/tests.zig`, 98 blocks) and
where its Rust descendant lives. Nothing is dropped; two entries moved layers
(noted inline). Line numbers are from the Zig source at the time of the port.

Non-test notes:

- `root.zig:11` `test {}` is a module aggregator, not a test — nothing to
  port.
- `tests.zig:1561-1565` is a comment-only note that Zig `cmdDep` prints to
  stderr; superseded by the folio stdout/stderr contract (see DECISIONS.md).

## ids (`src/ids_test.rs`)

| Zig (line) | Rust test |
|---|---|
| 85 shortHash produces 6-char hex string | `short_hash_is_six_lowercase_hex_chars` |
| 93 shortHash is deterministic | `short_hash_is_deterministic` |
| 99 shortHash different inputs produce different outputs | `short_hash_differs_for_different_inputs` |
| 105 shortHash empty input | `short_hash_empty_input` |
| 110 ULID generator produces 26-char string | `ulid_is_26_chars` |
| 118 ULID generator uses Crockford base32 alphabet | `ulid_uses_crockford_alphabet` |
| 129 ULID monotonic ordering within same millisecond | `ulid_monotonic_within_same_millisecond` |
| 142 ULID different timestamps produce different prefixes | `ulid_timestamp_prefix_differs_across_milliseconds` |
| 155 ULID multiple generations are all unique | `ulid_generations_are_all_unique` |

## Issue CRUD (`tests/store_crud.rs`)

| Zig (line) | Rust test |
|---|---|
| 178 createIssue returns ID with prefix | `create_issue_returns_id_with_prefix` |
| 189 createIssue and fetchIssue roundtrip | `create_and_fetch_roundtrip` |
| 208 createIssue default priority is stored | `create_issue_stores_defaults` |
| 223 createIssue with tags | `create_issue_with_tags` |
| 247 createIssue with empty tags | `create_issue_with_empty_tags` |
| 260 createIssue multiple issues get unique IDs | `create_issue_ids_are_unique` |
| 276 fetchIssue nonexistent returns IssueNotFound | `fetch_nonexistent_is_not_found` |
| 286 updateIssue changes title | `update_changes_title` |
| 300 updateIssue changes status | `update_changes_status` |
| 314 updateIssue changes priority | `update_changes_priority` |
| 328 updateIssue changes assignee | `update_changes_assignee` |
| 342 updateIssue changes body | `update_changes_body` |
| 356 updateIssue multiple fields at once | `update_multiple_fields_at_once` |
| 374 updateIssue adds tags | `update_adds_tags` |
| 390 updateIssue removes tags | `update_removes_tags` |
| 407 updateIssue add and remove tags simultaneously | `update_adds_and_removes_tags_simultaneously` |
| 425 updateIssue updates updated_at timestamp | `update_bumps_updated_at_and_preserves_created_at` |
| 449 updateIssue nonexistent returns IssueNotFound | `update_nonexistent_is_not_found` |
| 1013 updateIssue preserves unchanged fields | `update_preserves_unchanged_fields` |

## ID resolution (`tests/store_resolve.rs`)

| Zig (line) | Rust test |
|---|---|
| 459 resolveId exact match | `resolve_exact_match` |
| 471 resolveId prefix match | `resolve_prefix_match` |
| 485 resolveId nonexistent returns IssueNotFound | `resolve_nonexistent_is_not_found` |
| 493 resolveId ambiguous returns IssueIdAmbiguous | `resolve_ambiguous_prefix_errors` |
| 1440 resolveId with empty string matches all (ambiguous) | `resolve_empty_string_with_multiple_issues_is_ambiguous` |
| 1454 resolveId with single issue and empty prefix matches | `resolve_empty_string_with_single_issue_matches` |

## Comments (`tests/store_comments.rs`)

| Zig (line) | Rust test |
|---|---|
| 513 addComment returns ULID | `add_comment_returns_ulid` |
| 526 addComment multiple comments on same issue | `add_multiple_comments_on_same_issue` |
| 546 addComment on nonexistent issue returns IssueNotFound | `add_comment_on_nonexistent_issue_is_not_found` |
| 554 addComment with prefix ID resolution | `add_comment_resolves_prefix_id` |

## Dependencies (`tests/store_deps.rs`, one moved)

| Zig (line) | Rust test |
|---|---|
| 570 addDep blocks | `add_dep_blocks` |
| 582 addDep relates | `add_dep_relates` |
| 594 addDep invalid kind returns InvalidDepKind | **moved** → `src/model_test.rs::dep_kind_rejects_invalid_input` (closed `DepKind` enum; see DECISIONS.md) |
| 607 addDep self-dependency returns SelfDependency | `add_dep_to_self_errors` |
| 618 addDep nonexistent source returns IssueNotFound | `add_dep_nonexistent_source_is_not_found` |
| 629 addDep nonexistent target returns IssueNotFound | `add_dep_nonexistent_target_is_not_found` |
| 640 removeDep | `remove_dep` |
| 653 addDep duplicate is idempotent | `add_dep_duplicate_is_idempotent` |

## Prefix management (`tests/store_prefix.rs`)

| Zig (line) | Rust test |
|---|---|
| 668 setPrefix changes prefix | `set_prefix_changes_prefix` |
| 676 setPrefix affects new issue IDs | `set_prefix_affects_new_issue_ids` |
| 688 setPrefix persists across reopen | `set_prefix_persists_across_reopen` |

## JSONL persistence, reimport, recovery (`tests/store_jsonl.rs`)

| Zig (line) | Rust test |
|---|---|
| 715 createIssue writes to JSONL file | `create_issue_writes_snapshot_to_jsonl` |
| 732 addComment writes to JSONL file | `add_comment_writes_record_to_jsonl` |
| 749 addDep writes to JSONL file | `add_dep_writes_record_to_jsonl` |
| 767 removeDep writes removed state to JSONL | `remove_dep_appends_removed_record_to_jsonl` |
| 785 updateIssue appends new line to JSONL | `update_issue_appends_to_jsonl` |
| 805 forceReimport rebuilds SQLite from JSONL | `force_reimport_rebuilds_cache_from_jsonl` |
| 825 forceReimport with comments | `force_reimport_preserves_comments` |
| 843 forceReimport with dependencies | `force_reimport_preserves_dependencies` |
| 1069 store reopen recovers issues from JSONL | `reopen_without_cache_recovers_issues_from_jsonl` |
| 1123 store reopen recovers updated issue state | `reopen_without_cache_recovers_updated_state` |
| 1347 JSONL roundtrip preserves special chars after reimport | `reimport_preserves_special_chars` |
| 1363 JSONL roundtrip preserves tags after reimport | `reimport_preserves_tags` |
| 1404 writeJsonString escapes control characters | `jsonl_escapes_control_characters` |
| 1569 store opens with empty JSONL file | `opens_with_empty_jsonl` |
| 1575 store handles JSONL with trailing newlines | `opens_with_only_blank_lines` |
| 1593 store handles JSONL with malformed JSON lines | `opens_with_malformed_json_lines` |
| 1612 store handles JSONL with unknown record type | `opens_with_unknown_record_type` |
| 1630 store handles JSONL with missing required fields | `skips_records_missing_required_fields` |

No Zig ancestor (cache-behavior spec from the rewrite brief):
`reopen_imports_externally_appended_records`,
`reopen_after_jsonl_truncation_fully_reimports`,
`concurrent_handles_retry_through_busy`.

## FTS search (`tests/store_search.rs`, via `Store::search`)

| Zig (line) | Rust test |
|---|---|
| 1174 FTS search finds issue by title | `search_finds_issue_by_title` |
| 1189 FTS search finds issue by body | `search_finds_issue_by_body` |
| 1203 FTS search does not match unrelated terms | `search_does_not_match_unrelated_terms` |
| 1540 BUG: FTS search should find comment text after addComment | `search_finds_comment_text_without_reimport` |

## Edge cases (`tests/store_edge.rs`)

| Zig (line) | Rust test |
|---|---|
| 864 createIssue with special characters in title | `title_with_quotes_roundtrips` |
| 876 createIssue with newlines in body | `body_with_newlines_roundtrips` |
| 888 createIssue with backslash in title | `title_with_backslashes_roundtrips` |
| 900 createIssue with tab character in body | `body_with_tabs_roundtrips` |
| 912 addComment with special characters | `comment_with_special_chars_keeps_jsonl_valid` |
| 930 createIssue with unicode in title | `title_with_unicode_roundtrips` |
| 944 createIssue with priority 1 (highest) | `priority_one_roundtrips` |
| 956 createIssue with priority 5 (lowest) | `priority_five_roundtrips` |
| 968 createIssue with long title | `long_title_roundtrips` |
| 981 createIssue with long body | `long_body_roundtrips` |
| 994 createIssue with many tags | `ten_tags_roundtrip` |
| 1032 duplicate tag on same issue is idempotent | `duplicate_tag_collapses_to_one` |
| 1046 same tag shared across issues | `same_tag_shared_across_issues` |
| 1219 createIssue with priority 0 | `priority_zero_roundtrips` |
| 1231 createIssue with negative priority | `negative_priority_roundtrips` |
| 1243 createIssue with very large priority | `very_large_priority_roundtrips` |
| 1257 status transition open -> in_progress -> closed | `status_walks_open_in_progress_closed` |
| 1285 status transition closed -> open (reopen) | `status_reopens_from_closed` |
| 1302 removing nonexistent tag is a no-op | `removing_nonexistent_tag_is_noop` |
| 1321 createIssue with empty title | `empty_title_is_allowed` |
| 1333 createIssue with empty assignee | `empty_assignee_is_allowed` |
| 1380 rapid create and fetch cycle | `fifty_rapid_creates_all_fetchable` |
| 1423 updateIssue with very long body near buffer limit | `seven_thousand_byte_body_roundtrips` |
| 1470 BUG: createIssue with body exceeding 8192 byte JSONL buffer | `body_beyond_zig_buffer_limit_roundtrips` |
| 1487 BUG: addComment with long comment should not error | `long_comment_roundtrips` |
| 1501 BUG: appendDepJsonl buffer overflow with long IDs | `dep_write_with_generated_ids_succeeds` |
| 1517 BUG: negative priority stored and retrieved correctly | `negative_priority_survives_reimport` |

## Accounting

98 Zig tests → 98 Rust descendants (97 in place + 1 moved to the parse
boundary). Plus 5 Rust tests with no Zig ancestor (3 cache-behavior, 2
parse-boundary companions) and 9 `#[ignore = "planned feature"]` spec stubs
in `tests/planned_features.rs`: **112 tests total, 103 red + 9 ignored.**
