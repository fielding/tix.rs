//! Dependency edges between issues.

use crate::dep_kind::DepKind;
use crate::issue_id::IssueId;

/// An active dependency edge: `src` `kind`s `dst` (e.g. src blocks dst).
///
/// Removed edges do not exist here — removal is an event in the JSONL log,
/// not a state on the edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dep {
    pub src: IssueId,
    pub dst: IssueId,
    pub kind: DepKind,
}
