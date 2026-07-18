use super::*;

// No Zig ancestor: the Zig store never validated ids. The acceptance floor
// and its charset are a deliberate tightening backed by the 2026-07-17 audit
// of all 32 real stores (3024 records, zero violations); see DECISIONS.md.
#[test]
fn accepts_generated_and_legacy_dir_derived_ids() {
    for ok in ["tix-a3f01b", "tix.rs-9c2e44", "My_Repo-0f", "Nwallet-1d00c2", "a"] {
        assert!(ok.parse::<IssueId>().is_ok(), "{ok} should be accepted");
    }
}

#[test]
fn rejects_empty() {
    assert_eq!("".parse::<IssueId>(), Err(ParseIssueIdError::Empty));
}

#[test]
fn rejects_non_alphanumeric_edges() {
    assert_eq!("-abc".parse::<IssueId>(), Err(ParseIssueIdError::BadEdge));
    assert_eq!("abc-".parse::<IssueId>(), Err(ParseIssueIdError::BadEdge));
    assert_eq!(".abc".parse::<IssueId>(), Err(ParseIssueIdError::BadEdge));
}

#[test]
fn rejects_invalid_characters_and_names_the_culprit() {
    assert_eq!(
        "a b".parse::<IssueId>(),
        Err(ParseIssueIdError::InvalidChar(' '))
    );
    assert_eq!(
        "a\nb".parse::<IssueId>(),
        Err(ParseIssueIdError::InvalidChar('\n'))
    );
    assert_eq!(
        "a%b".parse::<IssueId>(),
        Err(ParseIssueIdError::InvalidChar('%'))
    );
}

#[test]
fn owned_and_borrowed_constructors_agree() {
    let via_parse: IssueId = "tix-a3f01b".parse().expect("valid id");
    let via_try_from = IssueId::try_from("tix-a3f01b".to_string()).expect("valid id");
    assert_eq!(via_parse, via_try_from);
}
