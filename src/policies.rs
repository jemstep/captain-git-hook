use crate::config::VerifyGitCommitsConfig;
use crate::fingerprints::*;
use crate::fs::*;
use crate::git::*;
use crate::gpg::*;

use git2::Oid;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::iter;
use std::path::PathBuf;
use std::time::Instant;

use log::*;

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
}

impl PolicyResult {
    pub fn and(self, res: PolicyResult) -> PolicyResult {
        match self {
            PolicyResult::Ok => res,
            x => x,
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

pub fn prepend_branch_name<F: Fs, G: Git>(
    commit_file: PathBuf,
) -> Result<PolicyResult, Box<dyn Error>> {
    info!("Executing policy: prepend_branch_name");

    let git = G::new()?;
    let branch = git.current_branch()?;
    F::prepend_string_to_file(branch, commit_file)?;
    Ok(PolicyResult::Ok)
}

pub fn verify_git_commits<G: Git, P: Gpg>(
    config: &VerifyGitCommitsConfig,
    old_value: &str,
    new_value: &str,
    ref_name: &str,
) -> Result<PolicyResult, Box<dyn Error>> {
    info!("Executing policy: verify_git_commits");
    let git = G::new()?;
    let start = Instant::now();
    let old_commit_id = Oid::from_str(old_value)?;
    let new_commit_id = Oid::from_str(new_value)?;

    let mut policy_result = PolicyResult::Ok;

    if new_commit_id.is_zero() {
        debug!("Delete branch detected, no commits to verify.")
    } else if git.is_tag(new_commit_id) {
        debug!("Tag detected, no commits to verify.")
    } else {
        let commits = commits_to_verify(&git, old_commit_id, new_commit_id)?;

        debug!("Number of commits to verify {} : ", commits.len());
        // for commit in &commits {
        //     G::debug_commit(&commit) // TODO
        // }

        let mut keyring = Fingerprint::read_fingerprints(&git, &config.team_fingerprints_file)?;

        let exclusions = find_and_verify_override_tags(
            &git,
            &commits,
            config.override_tags_required,
            &mut keyring,
        )?;
        let commits =
            commits_to_verify_with_exclusions(&git, old_commit_id, new_commit_id, exclusions)?;

        // TODO: This block needs to be inlined to wherever uses it
        let fingerprint_ids = keyring.values().map(|f| f.id.clone()).collect();
        if config.skip_recv_keys {
            debug!("Skipping importing GPG keys");
        } else {
            debug!("Fetching GPG public keys from {}", config.keyserver);

            if config.recv_keys_par {
                let _result = P::par_receive_keys(&config.keyserver, &fingerprint_ids)?;
            } else {
                let _result = P::receive_keys(&config.keyserver, &fingerprint_ids)?;
            }
        }

        if config.verify_email_addresses {
            policy_result = policy_result.and(verify_email_addresses(
                &config.author_domain,
                &config.committer_domain,
                &commits,
            ));
        }

        if config.verify_commit_signatures {
            policy_result =
                policy_result.and(verify_commit_signatures::<G>(&git, &commits, &keyring)?);
        }

        if config.verify_different_authors {
            policy_result = policy_result.and(verify_different_authors::<G>(
                &commits,
                &git,
                old_commit_id,
                new_commit_id,
                ref_name,
            )?);
        }
    }

    info!(
        "Policy verify_git_commits completed in: {} ms",
        start.elapsed().as_millis()
    );

    Ok(policy_result)
}

fn commits_to_verify<'a, G: Git>(
    git: &'a G,
    old_commit_id: Oid,
    new_commit_id: Oid,
) -> Result<Vec<VerificationCommit>, Box<dyn Error>> {
    // TODO fingerprints
    git.find_verification_commits(&[old_commit_id], &[new_commit_id], &HashMap::new())
}

fn commits_to_verify_with_exclusions<'a, G: Git>(
    git: &'a G,
    old_commit_id: Oid,
    new_commit_id: Oid,
    mut exclusions: Vec<Oid>,
) -> Result<Vec<VerificationCommit>, Box<dyn Error>> {
    // TODO fingerprints
    exclusions.push(old_commit_id);
    git.find_verification_commits(&exclusions, &[new_commit_id], &HashMap::new())
}

fn find_and_verify_override_tags<G: Git>(
    _git: &G,
    commits: &Vec<VerificationCommit>,
    required_tags: u8,
    _fingerprints: &mut HashMap<String, Fingerprint>,
) -> Result<Vec<Oid>, Box<dyn Error>> {
    // TODO: fetch keys

    Ok(commits
        .iter()
        .filter(|c| c.tags.len() >= required_tags.into())
        .filter(|_c| true) // TODO: verify all tags (this may fail)
        .map(|c| c.id)
        .collect())
}

fn verify_commit_signatures<G: Git>(
    git: &G,
    commits: &[VerificationCommit],
    fingerprints: &HashMap<String, Fingerprint>,
) -> Result<PolicyResult, Box<dyn Error>> {
    let verification_commits: HashMap<Oid, VerificationCommit> =
        generate_verification_commits(git, commits, fingerprints);

    commits.iter()
        .map(|commit| {
            let verification_commit = verification_commits.get(&commit.id);
            match verification_commit {
                Some(vc) => {
                    if vc.is_identical_tree {
                        info!("Signature verification passed for {}: verified identical to one of its parents, no signature required", commit.id);
                        Ok(PolicyResult::Ok)
                    } else if vc.valid_signature {
                        info!("Signature verification passed for {}: verified with a valid signature", commit.id);
                        Ok(PolicyResult::Ok)
                    } else if git.is_trivial_merge_commit(commit)? {
                        info!("Signature verification passed for {}: verified to be a trivial merge of its parents, no signature required", commit.id);
                        Ok(PolicyResult::Ok)
                    }  else {
                        error!("Signature verification failed for {}", commit.id);
                        if commit.is_merge_commit {
                            Ok(PolicyResult::UnsignedMergeCommit(commit.id))
                        } else {
                            Ok(PolicyResult::UnsignedCommit(commit.id))
                        }
                    }
                },
                None => {
                    error!("Signature verification failed for {}, verification commit not found", commit.id);
                    if commit.is_merge_commit {
                        Ok(PolicyResult::UnsignedMergeCommit(commit.id))
                    } else {
                        Ok(PolicyResult::UnsignedCommit(commit.id))
                    }
                }
            }
        })
        .collect()
}

fn generate_verification_commits<G: Git>(
    git: &G,
    commits: &[VerificationCommit],
    _fingerprints: &HashMap<String, Fingerprint>,
) -> HashMap<Oid, VerificationCommit> {
    let repo_path = git.path();

    let checked_verification_commits: HashMap<Oid, VerificationCommit> = commits
        .par_iter()
        .map(|commit| {
            let valid_signature = G::verify_commit_signature(repo_path, &commit);
            (
                commit.id,
                VerificationCommit {
                    valid_signature: valid_signature.unwrap_or(false),
                    ..commit.clone()
                },
            )
        })
        .collect();
    checked_verification_commits
}

fn verify_different_authors<G: Git>(
    commits: &[VerificationCommit],
    git: &G,
    old_commit_id: Oid,
    new_commit_id: Oid,
    ref_name: &str,
) -> Result<PolicyResult, Box<dyn Error>> {
    let new_commit = git.find_commit(new_commit_id)?;
    let new_branch = old_commit_id.is_zero();
    let is_merge = G::merge_commit(&new_commit);
    let is_head = git.is_head(ref_name)?;

    if !is_head {
        info!("Multiple author verification passed for {}: Not updating the head of the repo, does not require multiple authors", new_commit_id);
        Ok(PolicyResult::Ok)
    } else if !is_merge {
        info!("Multiple author verification passed for {}: Not a merge commit, does not require multiple authors", new_commit_id);
        Ok(PolicyResult::Ok)
    } else if new_branch {
        info!("Multiple author verification passed for {}: New branch does not require multiple authors for a merge commit", new_commit_id);
        Ok(PolicyResult::Ok)
    } else if commits.len() == 0 {
        info!("Multiple author verification passed for {}: No new commits pushed, does not require multiple authors", new_commit_id);
        Ok(PolicyResult::Ok)
    } else {
        let authors: HashSet<_> = commits
            .iter()
            .filter_map(|c| c.author_email.clone())
            .collect();
        if authors.len() <= 1 {
            error!(
                "Multiple author verification failed for {}: requires multiple authors, found {:?}",
                new_commit_id, authors
            );
            Ok(PolicyResult::NotEnoughAuthors(new_commit_id))
        } else {
            info!(
                "Multiple author verification passed for {}: found multiple authors, {:?}",
                new_commit_id, authors
            );
            Ok(PolicyResult::Ok)
        }
    }
}

fn verify_email_addresses(
    author_domain: &str,
    committer_domain: &str,
    commits: &[VerificationCommit],
) -> PolicyResult {
    commits
        .iter()
        .map(
            |commit| match (&commit.author_email, &commit.committer_email) {
                (None, _) => {
                    error!(
                        "Email address verification failed for {}: missing author email",
                        commit.id
                    );
                    PolicyResult::MissingAuthorEmail(commit.id)
                }
                (_, None) => {
                    error!(
                        "Email address verification failed for {}: missing committer email",
                        commit.id
                    );
                    PolicyResult::MissingCommitterEmail(commit.id)
                }
                (Some(s), _) if !s.ends_with(&format!("@{}", author_domain)) => {
                    error!(
                        "Email address verification failed for {}: invalid author email {}",
                        commit.id,
                        s.to_string()
                    );
                    PolicyResult::InvalidAuthorEmail(commit.id, s.to_string())
                }
                (_, Some(s)) if !s.ends_with(&format!("@{}", committer_domain)) => {
                    error!(
                        "Email address verification failed for {}: invalid committer email {}",
                        commit.id,
                        s.to_string()
                    );
                    PolicyResult::InvalidCommitterEmail(commit.id, s.to_string())
                }
                _ => {
                    info!("Email address verification passed for {}", commit.id);
                    PolicyResult::Ok
                }
            },
        )
        .collect()
}
