# Decisions

Deliberate deviations from the Zig tix (v0.1.0) behavior, plus the contracts
this rewrite pins. The Zig source at `fielding/tix` is the reference; the test
suite here is the spec. Anything not listed below is intended to match the Zig
semantics exactly.

## On-disk contract (unchanged — listed to make it explicit)

- `issues.jsonl` record shapes, key names, and status/kind wire strings are
  byte-compatible with the Zig writer; the Rust store must read existing
  Zig-written stores as-is. Timestamps are milliseconds since the Unix epoch.
- The derived cache keeps the file names `issues.db`, `issues.db-wal`,
  `issues.db-shm` inside the store dir, and deleting them must lose nothing
  (`tests/store_jsonl.rs` reopen tests).
- Self-healing rules: incremental import of new JSONL bytes from a stored
  offset; full rebuild when the file shrank; unparseable lines, records with
  missing required fields, and unknown record types are skipped, never a hard
  error. Unknown-type tolerance is also the forward-compatibility seam the
  planned `hold`/`delete` records rely on.
- Generated ids are `<prefix>-<short_hash(title + now_ms)>`; `short_hash` is
  FNV-1a 64 rendered as six lowercase hex chars taken nibble-wise from the
  least-significant end (must match the Zig `ids.zig` exactly, or prefix
  resolution against old stores breaks). Default prefix on first open is the
  store dir's parent directory name (`/repo/.tix` → `repo`), falling back to
  `"do"`.

## API shape (Rust idiom over Zig positional calls)

- `createIssue(title, body, pri, assignee, tags)` → `create_issue(&NewIssue)`
  and `updateIssue(id, 7 nullable args)` → `update_issue(id, &IssueUpdate)`.
  Same semantics, named fields instead of positional nulls.
- `update_issue` returns the resolved `IssueId` (Zig returned void and made
  the CLI re-resolve for printing).
- Typed ids (`IssueId`, `CommentId`) and closed enums (`Status`, `DepKind`,
  `HoldKind`) replace strings throughout the API. Wire names live in the
  enums' `as_str` and are pinned by tests.
- `Issue.tags` order is explicitly unspecified (the Zig tests already
  tolerated any order).
- Removed dependency edges are not represented in the API (`Dep` has no
  `state` field): removal is an event in the log, and `Store::deps` returns
  only active edges. The Zig sqlite schema's `state` column is an
  implementation choice, not a contract.

## Error model

- Per-module errors per the family Rust conventions: `store::Error` with
  `#[source]` chains; parse failures live on the model types
  (`ParseStatusError`, `ParseDepKindError`, `ParseHoldKindError`).
- **`InvalidDepKind` no longer exists at the store layer.** `DepKind` is a
  closed enum, so an invalid kind is unrepresentable past the parse boundary.
  The Zig test "addDep invalid kind returns InvalidDepKind" is ported as
  `src/model_test.rs::dep_kind_rejects_invalid_input`.
- `Error::DatabaseBusy` remains for lock contention that outlives the retry
  budget, but the store is expected to retry/busy-wait internally first
  (`tests/store_jsonl.rs::concurrent_handles_retry_through_busy`).
- New `Error::IssueIdExists` backs the planned `add --id` collision case.

## Test-port decisions

- **FTS tests query `Store::search`, not raw SQL.** The Zig tests prepared
  statements against `issues_fts` directly; the cache is an internal here.
  The behavioral assertions (title/body/comment matches, no unrelated
  matches, comment text searchable immediately after `add_comment` without a
  reimport) are all kept.
- **Zig buffer-overflow BUG tests became large-payload roundtrips.** The
  fixed 8192/4096/512-byte writer buffers do not exist in Rust; the size
  classes they guarded are still exercised (8100-byte body, 4000-char
  comment, dep writes).
- Tightened matchers where the Zig assertions were loose: search asserts the
  exact hit set, id-with-prefix asserts the actual store prefix, control-char
  escaping asserts the literal backslash-u0001 escape and the absence of the
  raw byte.
- Five tests have no Zig ancestor and are marked as such in comments:
  byte-offset incremental import of externally appended records, truncation
  triggering a full reimport, concurrent-handle busy retry (all three named
  in the rewrite brief), plus two parse-boundary companions in
  `src/model_test.rs`.
- The Zig suite's comment-only note about `cmdDep` printing to stderr is not
  ported: the folio contract (data on stdout, diagnostics on stderr) already
  supersedes it.

## CLI boundary (folio family)

- JSON envelope `{"schema_version":1,"data":…}` /
  `{"schema_version":1,"error":{"code","message"}}` under `--json`; stable
  snake_case error codes; exit codes 0 success / 1 operational / 2 usage
  (clap) / 3 not found / 4 ambiguity or semantic conflict / 5 validation.
  The code and exit maps are implemented in `src/cli.rs` as declarative
  tables.
- Priority is validated 1..=5 at the CLI only (clap range parser); the store
  accepts any `i32`, exactly like the Zig store (its tests use 0, -5, 99999).
- The Zig `-q/--quiet` and alias behavior (`new`, `ls`) are kept.

## Planned-feature event shapes (new record types, spec only)

Old readers (including the Zig tix) skip unknown record types, so these are
backward-compatible appends:

- Hold: `{"type":"hold","issue_id":…,"state":"active"|"released",
  "reason":…,"kind":"captain|external|load|parked|future",
  "until":<ms or null>,"created_at":<ms>}`. Held issues keep their status;
  `ready` excludes them. Hold kinds mirror the tasks-axi vocabulary.
- Delete: `{"type":"delete","issue_id":…,"deleted_at":<ms>}` — a tombstone.
  History before it is never rewritten; fetch/list/ready/search hide the
  issue afterward.
- Export is the JSONL event stream itself; import replays it (ids preserved,
  unknown types skipped). No separate interchange format.

## `dump` mapping (tasks-axi backlog.md grammar)

Golden file: `tests/golden/backlog.md`. Grammar source: tasks-axi 0.2.3
`markdown-grammar.js` (via the firstmate adapter-scout report §2.4).

- Sections `## In flight` / `## Queued` / `## Done` map from status
  `in_progress` / `open` / `closed`. Bullets are `- [ ] <id> - <title>`
  (section conveys state, firstmate-style) and `- [x]` under Done.
- Within a section: priority ascending, then `created_at` ascending, then id
  — the folio deterministic-ordering convention.
- Trailing tags in order: `(repo: …)` and `(kind: …)` from issue tags of the
  form `repo:<v>` / `kind:<v>`, `(since YYYY-MM-DD)` from `created_at`,
  `(closed YYYY-MM-DD)` from `updated_at` on Done rows, `(hold: <reason>)
  (hold-kind: <kind>)` for held issues. Dates are UTC.
- Body lines render as two-space-indented continuation lines.

## What is implemented vs. stubbed

Everything behavioral is `todo!()`. The only implemented code is
declarative: newtype accessors (`as_str`, `Display`), enum wire names,
`NewIssue`/`IssueUpdate` defaults, the CLI arg definitions (clap derive),
and the error-code/exit-code maps. No test required partial logic.

Known convention exception while the suite is red: `cli::execute` is a
`todo!()` on a user-reachable path, which the family Rust conventions
forbid for shipped binaries. Intentional here — the binary is not shipped
until the captain implements it; the panic *is* the red suite.
