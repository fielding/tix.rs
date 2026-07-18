//! Issue identifiers.

use std::fmt;
use std::str::FromStr;

/// An issue id: `<prefix>-<6-hex-hash>` when generated, or any caller-chosen
/// string for the planned `add --id`.
///
/// Whatever the producer — the generator, `--id`, or JSONL replay — every id
/// meets the same acceptance floor: ASCII `[A-Za-z0-9._-]`, non-empty,
/// alphanumeric at both ends. The charset covers every prefix real stores
/// derived from directory names (`tix.rs`, `Nwallet`, `justfielding.com`);
/// ids are byte-exact, with no case folding (see DECISIONS.md). Holding an
/// `IssueId` is proof the value passed validation: construction is only
/// possible through [`FromStr`] or [`TryFrom<String>`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IssueId(String);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ParseIssueIdError {
    #[error("issue id must not be empty")]
    Empty,
    #[error("issue id contains invalid character {0:?} (allowed: ASCII letters, digits, '.', '_', '-')")]
    InvalidChar(char),
    #[error("issue id must start and end with an ASCII letter or digit")]
    BadEdge,
}

// Private so call sites cannot keep a "checked" raw string: the only way to
// know a string passed is to hold an IssueId.
fn validate(s: &str) -> Result<(), ParseIssueIdError> {
    let first = s.chars().next().ok_or(ParseIssueIdError::Empty)?;
    let last = s.chars().last().expect("non-empty checked above");
    if let Some(c) = s
        .chars()
        .find(|&c| !c.is_ascii_alphanumeric() && !matches!(c, '.' | '_' | '-'))
    {
        return Err(ParseIssueIdError::InvalidChar(c));
    }
    if !first.is_ascii_alphanumeric() || !last.is_ascii_alphanumeric() {
        return Err(ParseIssueIdError::BadEdge);
    }
    Ok(())
}

impl FromStr for IssueId {
    type Err = ParseIssueIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate(s)?;
        Ok(Self(s.to_owned()))
    }
}

impl TryFrom<String> for IssueId {
    type Error = ParseIssueIdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        validate(&s)?;
        Ok(Self(s))
    }
}

impl IssueId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IssueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
#[path = "issue_id_test.rs"]
mod tests;
