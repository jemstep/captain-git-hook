use crate::git::*;
use crate::gpg::*;
use crate::fs::*;
use crate::config::VerifyGitCommitsConfig;
use crate::pretty::*;
use crate::fingerprints::*;

use git2::{Commit, Oid};
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::iter;

use log::*;

#[derive(Debug, Clone)]
pub enum PolicyResult {
    Ok,
    UnsignedCommit(Oid),
    NotEnoughAuthors(Oid),
    InvalidAuthorEmail(Oid, String),
    MissingAuthorEmail(Oid),
    InvalidCommitterEmail(Oid, String),
    MissingCommitterEmail(Oid),
}

impl PolicyResult {
    pub fn and(self, res: PolicyResult) -> PolicyResult {
        match self {
            PolicyResult::Ok => res,
            x => x
        }
    }
    pub fn is_ok(&self) -> bool {
        match self {
            PolicyResult::Ok => true,
            _ => false
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
            NotEnoughAuthors(id) => write!(f, "Merge commit needs to have multiple authors in the branch: {}", id),
            InvalidAuthorEmail(id, email) => write!(f, "Commit has an invalid author email ({}): {}", email, id),
            MissingAuthorEmail(id) => write!(f, "Commit does not have an author email: {}", id),
            InvalidCommitterEmail(id, email) => write!(f, "Commit has an invalid committer email ({}): {}", email, id),
            MissingCommitterEmail(id) => write!(f, "Commit does not have a committer email: {}", id),
        }
    }
}

impl iter::FromIterator<PolicyResult> for PolicyResult {
    fn from_iter<I: IntoIterator<Item=PolicyResult>>(iter: I) -> Self {
        iter.into_iter()
            .find(PolicyResult::is_err)
            .unwrap_or(PolicyResult::Ok)
    }
}

pub fn prepend_branch_name<F: Fs, G: Git>(commit_file: PathBuf) -> Result<PolicyResult, Box<dyn Error>> {
    debug!("Executing policy: prepend_branch_name");
    
    let git = G::new()?;
    let branch = git.current_branch()?;
    F::prepend_string_to_file(branch, commit_file)?;
    Ok(PolicyResult::Ok)
}

pub fn verify_git_commits<G: Git, P: Gpg>(config: &VerifyGitCommitsConfig, old_value: &str, new_value: &str) -> Result<PolicyResult, Box<dyn Error>> {
    info!("{}", seperator("verify_git_commits STARTED"));
    let git = G::new()?;
    let start = Instant::now();
    let old_commit_id = Oid::from_str(old_value)?;
    let new_commit_id = Oid::from_str(new_value)?;

    let mut policy_result = PolicyResult::Ok;
        
    if new_commit_id.is_zero() {
        info!("{}", block("DELETE BRANCH detected, no commits to verify."))
    } else if git.is_tag(new_commit_id) {
        info!("{}", block("TAG detected, no commits to verify."))
    } else {
        let new_commit = git.find_commit(new_commit_id)?;
        let new_branch = old_commit_id.is_zero();
        let merging = G::merge_commit(&new_commit) && !new_branch;
        let commits = commits_to_verify(&git, old_commit_id, new_commit_id)?;

        info!("Number of commits to verify {} : ", commits.len());
        for commit in &commits { G::debug_commit(&commit) };
        info!("{}", seperator(""));

        let commit_fingerprints = git.find_commit_fingerprints(&config.team_fingerprints_file, &commits)?;
        let fingerprint_ids = commit_fingerprints.values().map(|f| f.id.clone()).collect();

        if config.skip_recv_keys {
            info!("Skipping importing GPG keys");
        } else {
            info!("Find fingerprints for commits, and receive associated gpg keys");
            
            if config.recv_keys_par {
                let _result = P::par_receive_keys(&config.keyserver,&fingerprint_ids)?;
            } else {
                let _result = P::receive_keys(&config.keyserver,&fingerprint_ids)?;
            }
        }
        
        info!("{}", seperator(""));
        if config.verify_email_addresses {
            policy_result = policy_result.and(verify_email_addresses(&config.author_domain, &config.committer_domain, &commits));
            info!("{}", seperator(""));
        }
       
        if config.verify_commit_signatures {
            policy_result = policy_result.and(verify_commit_signatures::<G>(&git, &commits, &commit_fingerprints)?);
            info!("{}", seperator(""));
        }
        
        if merging && config.verify_different_authors {
            policy_result = policy_result.and(verify_different_authors::<G>(&commits, new_commit_id));
            info!("{}", seperator(""));
        }
    }

    let duration = start.elapsed();

    info!("verify_git_commits COMPLETED in: {} ms", duration.as_millis());

    Ok(policy_result)
}

fn commits_to_verify<'a, G: Git>(git: &'a G, old_commit_id: Oid, new_commit_id: Oid) -> Result<Vec<Commit<'a>>, Box<dyn Error>>  {
    if old_commit_id.is_zero() {
        debug!("{}", block("NEW BRANCH detected"));
        git.find_unpushed_commits(new_commit_id)
    } else {
        git.find_commits(old_commit_id, new_commit_id)
    }
}


fn verify_commit_signatures<G: Git>(git: &G, commits: &Vec<Commit<'_>>, fingerprints: &HashMap<String, Fingerprint>) -> Result<PolicyResult, Box<dyn Error>> {
    info!("Verify commit signatures");
    commits.iter()
        .map(|commit| {
            if G::is_identical_tree_to_any_parent(commit) {
                debug!("{}: verified identical to one of its parents, no signature required", commit.id());
                Ok(PolicyResult::Ok)
            } else if git.is_trivial_merge_commit(commit) {
                debug!("{}: verified to be a trivial merge of its parents, no signature required", commit.id());
                Ok(PolicyResult::Ok)
            } else {
                match git.verify_commit_signature(commit, fingerprints) {
                    Ok(true) => {
                        debug!("{}: verified with a valid signature", commit.id());
                        Ok(PolicyResult::Ok)
                    },
                    Ok(false) => {
                        debug!("{}: unverified, requies a valid signature", commit.id());
                        Ok(PolicyResult::UnsignedCommit(commit.id()))
                    },
                    Err(e) => Err(e)
                }
            }
        })
        .collect()
}

fn verify_different_authors<G: Git>(commits: &Vec<Commit<'_>>, id: Oid) -> PolicyResult {
    info!("Verify multiple authors");
    let authors : HashSet<_> = commits.iter().filter_map(|c| {
        c.author().email().map(|e| e.to_string())
    }).collect();
    debug!("Author set: {:#?}", authors);
    if authors.len() <= 1 {
        PolicyResult::NotEnoughAuthors(id)
    } else {
        PolicyResult::Ok
    }
}

fn verify_email_addresses(author_domain: &str,committer_domain: &str, commits: &Vec<Commit<'_>>) -> PolicyResult {
    info!("Verify email addresses");
    commits.iter()
        .map(|commit| {
            debug!("Verify author, committer email addresses for commit {}", commit.id());
            match (commit.author().email(), commit.committer().email()) {
                (None, _) => PolicyResult::MissingAuthorEmail(commit.id()),
                (_, None) => PolicyResult::MissingCommitterEmail(commit.id()),
                (Some(s), _) if !s.ends_with(&format!("@{}", author_domain)) => PolicyResult::InvalidAuthorEmail(commit.id(), s.to_string()),
                (_, Some(s)) if !s.ends_with(&format!("@{}", committer_domain)) => PolicyResult::InvalidCommitterEmail(commit.id(), s.to_string()),
                _ => PolicyResult::Ok
            }
        })
        .collect()
}

