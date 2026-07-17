# Project agent memory

This file is the project's committed home for project-intrinsic agent knowledge: build, test, release, architecture, and sharp-edge notes that should travel with the code.

- This repo is intentionally a **red test suite**: every API body is `todo!()`
  and `cargo test` failing 103 tests is the correct baseline state. Never
  "fix" red tests by weakening assertions; the implementation is the captain's.
  README "The red suite" has the work-down order.
- Contracts and deviations from the Zig reference live in `DECISIONS.md`;
  the Zig→Rust test map is `docs/zig-test-inventory.md`. The Zig original at
  `~/src/hack/tix` and folio at `~/Downloads/folio` are read-only references.
- The `issues.jsonl` record shapes and `issues.db*` cache file names are
  cross-implementation contracts (existing Zig-written stores must load);
  don't rename fields or files without updating DECISIONS.md.
- Rust style follows the user-level `rust-conventions` skill (per-module
  errors, `#[source]` not `#[from]`, `.expect` not `.unwrap`, sibling
  `<module>_test.rs` files via `#[path]`).
- Keep `cargo clippy --all-targets` at zero warnings; the ignored
  planned-feature stubs in `tests/planned_features.rs` stay `#[ignore]` until
  their feature lands.

## Maintaining this file

Keep this file for knowledge useful to almost every future agent session in this project.
Do not repeat what the codebase already shows; point to the authoritative file or command instead.
Prefer rewriting or pruning existing entries over appending new ones.
When updating this file, preserve this bar for all agents and keep entries concise.
