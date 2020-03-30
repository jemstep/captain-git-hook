use git2::Oid;
use std::error::Error;
use std::fmt;
use std::iter;

#[derive(Debug, Clone)]
pub enum PolicyResult {
    Ok,
    UnsignedCommit(Oid),
    UnsignedMergeCommit(Oid),
    NotEnoughAuthors(Oid),
    InvalidAuthorEmail(Oid, String),
    MissingAuthorEmail(Oid),
    InvalidCommitterEmail(Oid, String),
    MissingCommitterEmail(Oid),
    NotRebased(Oid),
}

impl PolicyResult {
    pub fn and(self, res: PolicyResult) -> PolicyResult {
        match self {
            PolicyResult::Ok => res,
            x => x,
        }
    }
    pub fn and_then(
        self,
        mut next: impl FnMut() -> Result<PolicyResult, Box<dyn Error>>,
    ) -> Result<PolicyResult, Box<dyn Error>> {
        match self {
            PolicyResult::Ok => next(),
            x => Ok(x),
        }
    }
    pub fn is_ok(&self) -> bool {
        match self {
            PolicyResult::Ok => true,
            _ => false,
        }
    }
    pub fn is_err(&self) -> bool {
        !self.is_ok()
    }
}

impl fmt::Display for PolicyResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use PolicyResult::*;

        match self {
            Ok => write!(f, "Ok"),
            UnsignedCommit(id) => write!(f, "Commit does not have a valid GPG signature: {}", id),
            UnsignedMergeCommit(id) => write!(f, "Commit does not have a valid GPG signature: {}. This is a merge commit, please note that if there were conflicts that needed to be resolved then the commit needs a signature.", id),
            NotEnoughAuthors(id) => write!(f, "Merge commit needs to have multiple authors in the branch: {}", id),
            InvalidAuthorEmail(id, email) => write!(f, "Commit has an invalid author email ({}): {}", email, id),
            MissingAuthorEmail(id) => write!(f, "Commit does not have an author email: {}", id),
            InvalidCommitterEmail(id, email) => write!(f, "Commit has an invalid committer email ({}): {}", email, id),
            MissingCommitterEmail(id) => write!(f, "Commit does not have a committer email: {}", id),
            NotRebased(id) => write!(f, "Merge commit needs to be rebased on the mainline before it can be merged: {}", id)
        }
    }
}

impl iter::FromIterator<PolicyResult> for PolicyResult {
    fn from_iter<I: IntoIterator<Item = PolicyResult>>(iter: I) -> Self {
        iter.into_iter()
            .find(PolicyResult::is_err)
            .unwrap_or(PolicyResult::Ok)
    }
}
