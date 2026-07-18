use super::*;

// Zig: "addDep invalid kind returns InvalidDepKind" — dep-kind validity is now
// a parse-boundary concern: `DepKind` is a closed enum, so an invalid kind
// cannot reach `Store::add_dep` at all (see DECISIONS.md).
#[test]
fn dep_kind_rejects_invalid_input() {
    assert!("depends".parse::<DepKind>().is_err());
}

// Companion to the above: the two valid wire names round-trip.
#[test]
fn dep_kind_parses_wire_names() {
    assert_eq!(
        "blocks".parse::<DepKind>().expect("blocks is a valid kind"),
        DepKind::Blocks
    );
    assert_eq!(
        "relates"
            .parse::<DepKind>()
            .expect("relates is a valid kind"),
        DepKind::Relates
    );
}
