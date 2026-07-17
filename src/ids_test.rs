use std::collections::HashSet;

use super::*;

const CROCKFORD: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

// Zig: "shortHash produces 6-char hex string"
#[test]
fn short_hash_is_six_lowercase_hex_chars() {
    let hash = short_hash("hello world");
    assert_eq!(hash.len(), 6);
    assert!(
        hash.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "not lowercase hex: {hash:?}"
    );
}

// Zig: "shortHash is deterministic"
#[test]
fn short_hash_is_deterministic() {
    assert_eq!(short_hash("test input"), short_hash("test input"));
}

// Zig: "shortHash different inputs produce different outputs"
#[test]
fn short_hash_differs_for_different_inputs() {
    assert_ne!(short_hash("input one"), short_hash("input two"));
}

// Zig: "shortHash empty input"
#[test]
fn short_hash_empty_input() {
    assert_eq!(short_hash("").len(), 6);
}

// Zig: "ULID generator produces 26-char string"
#[test]
fn ulid_is_26_chars() {
    let mut generator = UlidGenerator::from_seed(42);
    assert_eq!(generator.next(1000).len(), 26);
}

// Zig: "ULID generator uses Crockford base32 alphabet"
#[test]
fn ulid_uses_crockford_alphabet() {
    let mut generator = UlidGenerator::from_seed(42);
    let ulid = generator.next(1000);
    assert!(
        ulid.chars().all(|c| CROCKFORD.contains(c)),
        "non-Crockford char in {ulid:?}"
    );
}

// Zig: "ULID monotonic ordering within same millisecond"
#[test]
fn ulid_monotonic_within_same_millisecond() {
    let mut generator = UlidGenerator::from_seed(42);
    let a = generator.next(1000);
    let b = generator.next(1000);
    // Same timestamp prefix, so lexicographic order is generation order.
    assert!(a < b, "expected {a:?} < {b:?}");
}

// Zig: "ULID different timestamps produce different prefixes"
#[test]
fn ulid_timestamp_prefix_differs_across_milliseconds() {
    let mut generator = UlidGenerator::from_seed(42);
    let a = generator.next(1000);
    let b = generator.next(2000);
    // The first 10 chars encode the 48-bit timestamp.
    assert_ne!(a[..10], b[..10]);
}

// Zig: "ULID multiple generations are all unique"
#[test]
fn ulid_generations_are_all_unique() {
    let mut generator = UlidGenerator::from_seed(99);
    let ulids: HashSet<String> = (0..20_u64).map(|i| generator.next(5000 + i)).collect();
    assert_eq!(ulids.len(), 20);
}
