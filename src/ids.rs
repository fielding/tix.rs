//! Identifier generation.
//!
//! Issue ids are `<store-prefix>-<short_hash(title + created_at_ms)>`
//! (e.g. `demo-daffe9`). Comment ids are monotonic ULIDs.
#![expect(unused_variables, reason = "todo!() stubs; remove once implemented")]

/// Six lowercase hex characters derived from an FNV-1a 64-bit hash of
/// `content`, taking the low 4 bits first (nibble-at-a-time from the least
/// significant end, matching the Zig implementation byte-for-byte so ids in
/// existing stores stay stable).
pub fn short_hash(content: &str) -> String {
    todo!()
}

/// Monotonic ULID generator: 26 characters of Crockford base32
/// (`0123456789ABCDEFGHJKMNPQRSTVWXYZ`), 48-bit millisecond timestamp prefix,
/// 80-bit random tail.
///
/// Fields are the implementer's choice; the observable contract is pinned by
/// `ids_test.rs`.
pub struct UlidGenerator;

impl UlidGenerator {
    /// A generator whose random tail is seeded deterministically; the same
    /// seed and timestamp sequence must reproduce the same ULID sequence.
    pub fn from_seed(seed: u64) -> Self {
        todo!()
    }

    /// The next ULID for the given millisecond timestamp.
    ///
    /// When `timestamp_ms` does not advance past the previous call, the
    /// 80-bit tail increments instead of re-randomizing, so ULIDs generated
    /// within one millisecond still sort strictly ascending.
    pub fn next(&mut self, timestamp_ms: u64) -> String {
        todo!()
    }
}

#[cfg(test)]
#[path = "ids_test.rs"]
mod tests;
