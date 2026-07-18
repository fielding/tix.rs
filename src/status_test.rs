use super::*;

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
        "closed"
            .parse::<Status>()
            .expect("closed is a valid status"),
        Status::Closed
    );
    assert!("done".parse::<Status>().is_err());
}
