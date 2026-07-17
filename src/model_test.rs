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
        "relates".parse::<DepKind>().expect("relates is a valid kind"),
        DepKind::Relates
    );
}

// No Zig ancestor: main.zig's isValidStatus covers this at the CLI; here the
// closed Status enum owns it. Pins the wire names and rejection behavior.
#[test]
fn status_parses_wire_names_and_rejects_junk() {
    assert_eq!(
        "open".parse::<Status>().expect("open is a valid status"),
        Status::Open
    );
    assert_eq!(
        "in_progress"
            .parse::<Status>()
            .expect("in_progress is a valid status"),
        Status::InProgress
    );
    assert_eq!(
        "closed".parse::<Status>().expect("closed is a valid status"),
        Status::Closed
    );
    assert!("done".parse::<Status>().is_err());
}
