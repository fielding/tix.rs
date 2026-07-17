# tix.rs

Rust rewrite of tix (https://github.com/fielding/tix), designed for the folio/tix/pledge family.

Test suite ported from the Zig implementation first; implementation follows (TDD).

## The red suite

This repo is currently a **spec, not a program**: the full Zig test suite is
ported to Rust and every public API body is `todo!()`, so the suite compiles
and fails red by design. `docs/zig-test-inventory.md` maps all 98 Zig tests to
their Rust descendants; `DECISIONS.md` records every deliberate deviation and
the contracts (JSONL format, cache self-healing, CLI envelope) the tests pin.

Current state: 112 tests — 103 red on `todo!()` panics, 9
`#[ignore = "planned feature"]` spec stubs for features the Zig tix doesn't
have (`add --id`, hold/defer, export/import, delete-as-tombstone, `dump` to
the tasks-axi backlog grammar).

### Working it down

```sh
cargo test                    # the scoreboard: red count only ever shrinks
cargo test --lib              # start here: ids, then model parsing
cargo test --test store_crud  # then the store, roughly in this order:
                              #   store_crud → store_resolve → store_comments
                              #   → store_deps → store_prefix → store_jsonl
                              #   → store_search → store_edge
cargo test -- --ignored       # planned features, one at a time, when ready
cargo clippy --all-targets    # kept clean throughout
```

Suggested path: implement `src/ids.rs` first (pure functions, 9 tests green,
and `short_hash` must match the Zig bit-for-bit — see DECISIONS.md), then the
`FromStr` impls in `src/model.rs`, then `Store::open`/`create_issue`/
`fetch_issue`/`resolve_id` (which flips most of the suite), then the JSONL
append/import paths, then search/ready/list, and finally `cli::execute`. Each
module carries `#![expect(unused_variables)]` for its stub parameters — remove
it as you implement and the compiler will tell you when it's no longer earned.

The `tests/` files state which Zig test each case descends from; if a red test
looks wrong, check it against the Zig original and `DECISIONS.md` before
changing the assertion.
