//! Identifier generation.
//!
//! Issue ids are `<store-prefix>-<short_hash(title + created_at_ms)>`
//! (e.g. `demo-daffe9`). Comment ids are monotonic ULIDs.

/// Six lowercase hex characters derived from an FNV-1a 64-bit hash of
/// `content`, taking the low 4 bits first (nibble-at-a-time from the least
/// significant end, matching the Zig implementation byte-for-byte so ids in
/// existing stores stay stable).
pub fn short_hash(content: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;

    for byte in content.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }

    let hex = b"0123456789abcdef";
    let mut out = String::with_capacity(6);
    for _ in 0..6 {
        out.push(char::from(hex[(hash & 0xf) as usize]));
        hash >>= 4;
    }
    out
}

/// Monotonic ULID generator: 26 characters of Crockford base32
/// (`0123456789ABCDEFGHJKMNPQRSTVWXYZ`), 48-bit millisecond timestamp prefix,
/// 80-bit random tail.
///
/// Fields are the implementer's choice; the observable contract is pinned by
/// `ids_test.rs`.
pub struct UlidGenerator {
    state: u64,
    last_ms: u64,
    last_rand: u128,
}

impl UlidGenerator {
    /// A generator whose random tail is seeded deterministically; the same
    /// seed and timestamp sequence must reproduce the same ULID sequence.
    pub fn from_seed(seed: u64) -> Self {
        Self {
            state: seed,
            last_ms: 0,
            last_rand: 0,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// The next ULID for the given millisecond timestamp.
    ///
    /// When `timestamp_ms` does not advance past the previous call, the
    /// 80-bit tail increments instead of re-randomizing, so ULIDs generated
    /// within one millisecond still sort strictly ascending.s
    pub fn next(&mut self, timestamp_ms: u64) -> String {
        let ts = timestamp_ms & 0xFFFF_FFFF_FFFF;

        let rand80: u128 = if ts > self.last_ms {
            let hi = u128::from(self.next_u64());
            let lo = u128::from(self.next_u64() & 0xFFFF);
            (hi << 16) | lo
        } else {
            (self.last_rand + 1) & ((1u128 << 80) - 1)
        };
        self.last_ms = ts;
        self.last_rand = rand80;

        let value = (u128::from(ts) << 80) | rand80;

        const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        let mut out = String::with_capacity(26);
        for shift in (0..26).rev() {
            let idx = ((value >> (5 * shift)) & 0x1F) as usize;
            out.push(char::from(ALPHABET[idx]));
        }
        out
    }
}

#[cfg(test)]
#[path = "ids_test.rs"]
mod tests;
