pub mod policy_result;

use crate::config::VerifyGitCommitsConfig;
use crate::fs::*;
use crate::git::*;
use crate::gpg::*;
use crate::keyring::*;
use crate::reference_update::ReferenceUpdate;

use self::policy_result::PolicyResult;

use git2::Oid;
use rayon::prelude::*;
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use log::*;

pub fn prepend_branch_name<F: Fs, G: Git>(
    git: &G,
    commit_file: PathBuf,
) -> Result<PolicyResult, Box<dyn Error>> {
    info!("Executing policy: prepend_branch_name");

    let branch = git.current_branch()?;
    F::prepend_string_to_file(branch, commit_file)?;
    Ok(PolicyResult::Ok)
}

pub fn verify_git_commits<G: Git, P: Gpg>(
    git: &G,
    gpg: P,
    config: &VerifyGitCommitsConfig,
    ref_update: &ReferenceUpdate,
) -> Result<PolicyResult, Box<dyn Error>> {
    info!("Executing policy: verify_git_commits");
    let start = Instant::now();

    let mut policy_result = PolicyResult::Ok;

    if let ReferenceUpdate::Delete { .. } = ref_update {
        debug!("Delete branch detected, no commits to verify.")
    } else if git.is_tag(ref_update.ref_name())? {
        debug!("Tag detected, no commits to verify.")
    } else {
        let all_commits = commits_to_verify(git, &ref_update, &config.override_tag_pattern)?;

        debug!("Number of commits to verify {} : ", all_commits.len());
        for commit in &all_commits {
            debug!("{:?}", commit);
        }

        let mut keyring =
            Keyring::from_team_fingerprints_file(git.read_file(&config.team_fingerprints_file)?);

        let manually_verified_commmits = find_and_verify_override_tags(
            git,
            &gpg,
            &all_commits,
            config.override_tags_required,
            &mut keyring,
        )?;
        let not_manually_verified_commits = commits_to_verify_excluding_manually_verified(
            git,
            &ref_update,
            manually_verified_commmits,
            &config.override_tag_pattern,
        )?;

        if config.verify_email_addresses {
            policy_result = policy_result.and(verify_email_addresses(
                &config.author_domain,
                &config.committer_domain,
                &not_manually_verified_commits,
            ));
        }

        if config.verify_commit_signatures {
            policy_result = policy_result.and(verify_commit_signatures::<G, P>(
                git,
                &gpg,
                &not_manually_verified_commits,
                &mut keyring,
            )?);
        }

        if config.verify_different_authors {
            policy_result = policy_result.and(verify_different_authors::<G>(
                &all_commits,
                git,
                &ref_update,
            )?);
        }

        if config.verify_rebased {
            policy_result = policy_result.and(verify_rebased::<G>(
                &all_commits,
                git,
                &ref_update,
                &config.override_tag_pattern,
            )?);
        }
    }

    info!(
        "Policy verify_git_commits completed in: {} ms",
        start.elapsed().as_millis()
    );

    Ok(policy_result)
}

fn commits_to_verify<G: Git>(
    git: &G,
    ref_update: &ReferenceUpdate,
    override_tag_pattern: &Option<String>,
) -> Result<Vec<Commit>, Box<dyn Error>> {
    let to_exclude = ref_update.old_commit_id().into_iter().collect::<Vec<_>>();
    let to_include = ref_update.new_commit_id().into_iter().collect::<Vec<_>>();
    git.find_new_commits(&to_exclude, &to_include, override_tag_pattern)
}

fn commits_to_verify_excluding_manually_verified<G: Git>(
    git: &G,
    ref_update: &ReferenceUpdate,
    manually_verified: Vec<Oid>,
    override_tag_pattern: &Option<String>,
) -> Result<Vec<Commit>, Box<dyn Error>> {
    let mut to_exclude = manually_verified;
    if let Some(old_commit_id) = ref_update.old_commit_id() {
        to_exclude.push(old_commit_id);
    }
    let to_include = ref_update.new_commit_id().into_iter().collect::<Vec<_>>();
    git.find_new_commits(&to_exclude, &to_include, override_tag_pattern)
}

fn find_and_verify_override_tags<G: Git, P: Gpg>(
    git: &G,
    gpg: &P,
    commits: &Vec<Commit>,
    required_tags: u8,
    keyring: &mut Keyring,
) -> Result<Vec<Oid>, Box<dyn Error>> {
    let repo_path = git.path();
    gpg.receive_keys(
        keyring,
        &commits
            .iter()
            .filter(|c| c.tags.len() >= required_tags.into())
            .flat_map(|c| c.tags.iter().flat_map(|t| t.tagger_email.as_deref()))
            .collect(),
    )?;

    let tagged_commits = commits
        .iter()
        .filter(|c| c.tags.len() >= required_tags.into())
        .filter_map(|c| {
            let verified_taggers = c
                .tags
                .iter()
                .filter(|t| verify_tag_logging_errors::<G>(&repo_path, t, keyring))
                .filter_map(|t| t.tagger_email.as_ref())
                .collect::<HashSet<_>>();

            if verified_taggers.len() >= required_tags.into() {
                info!("Override tags found for {}. Tags created by {:?}. This commit, and it's ancestors, do not require validation.", c.id, verified_taggers);
                Some(c.id)
            } else {
                None
            }
        })
        .collect();

    Ok(tagged_commits)
}

fn verify_tag_logging_errors<G: Git>(
    repo_path: &std::path::Path,
    tag: &Tag,
    keyring: &Keyring,
) -> bool {
    match G::verify_tag_signature(repo_path, tag, keyring) {
        Ok(result) => result,
        Err(e) => {
            error!(
                "Technical error occurred while trying to validate tag {}. Error: {}",
                tag.name, e
            );
            false
        }
    }
}

fn verify_commit_signatures<G: Git, P: Gpg>(
    git: &G,
    gpg: &P,
    commits: &[Commit],
    keyring: &mut Keyring,
) -> Result<PolicyResult, Box<dyn Error>> {
    gpg.receive_keys(
        keyring,
        &commits
            .iter()
            .filter_map(|c| c.committer_email.as_deref())
            .collect(),
    )?;

    let repo_path = git.path();
    let commits_with_verified_signatures: HashSet<Oid> = commits
        .par_iter()
        .filter(|commit| {
            match G::verify_commit_signature(repo_path, &commit, keyring) {
                Ok(result) => result,
                Err(e) => {
                    error!(
                        "Technical error occurred while trying to validate commit signature {}. Error: {}",
                           commit.id, e
                    );
                    false
                }
            }
        })
        .map(|commit| commit.id)
        .collect();

    commits.iter()
        .map(|commit| {
            if commit.is_identical_tree_to_any_parent {
                info!("Signature verification passed for {}: verified identical to one of its parents, no signature required", commit.id);
                Ok(PolicyResult::Ok)
            } else if commits_with_verified_signatures.contains(&commit.id) {
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
        })
        .collect()
}

fn verify_different_authors<G: Git>(
    commits: &[Commit],
    git: &G,
    ref_update: &ReferenceUpdate,
) -> Result<PolicyResult, Box<dyn Error>> {
    match ref_update {
        ReferenceUpdate::Delete { .. } => {
            info!("Multiple author verification passed: No checks required for deleting a branch");
            Ok(PolicyResult::Ok)
        }
        ReferenceUpdate::New { new_commit_id, .. } => {
            info!("Multiple author verification passed for {}: New branch does not require multiple authors for a merge commit", new_commit_id);
            Ok(PolicyResult::Ok)
        }
        ReferenceUpdate::Update {
            new_commit_id,
            ref_name,
            ..
        } => {
            let is_merge = git.is_merge_commit(*new_commit_id);
            let is_mainline = git.is_mainline(ref_name)?;

            if !is_mainline {
                info!("Multiple author verification passed for {}: Not updating a mainline branch, does not require multiple authors", new_commit_id);
                Ok(PolicyResult::Ok)
            } else if !is_merge {
                info!("Multiple author verification passed for {}: Not a merge commit, does not require multiple authors", new_commit_id);
                Ok(PolicyResult::Ok)
            } else if commits.len() == 0 {
                info!("Multiple author verification passed for {}: No new commits pushed, does not require multiple authors", new_commit_id);
                Ok(PolicyResult::Ok)
            } else if commits.len() == 1 && commits[0].is_identical_tree_to_any_parent {
                info!("Multiple author verification passed for {}: There is only one commit and it has an identical filetree to one of its parents", new_commit_id);
                Ok(PolicyResult::Ok)
            } else if commits.len() == 1 && git.is_trivial_merge_commit(&commits[0])? {
                info!("Multiple author verification passed for {}: There is only one commit and it is a trivial merge between mainline branches", new_commit_id);
                Ok(PolicyResult::Ok)
            } else {
                let authors: HashSet<_> = commits
                    .iter()
                    .flat_map(|c| {
                        c.tags
                            .iter()
                            .filter_map(|t| t.tagger_email.as_ref())
                            .chain(c.author_email.as_ref())
                    })
                    .collect();
                if authors.len() <= 1 {
                    error!(
                "Multiple author verification failed for {}: requires multiple authors, found {:?}",
                new_commit_id, authors
            );
                    Ok(PolicyResult::NotEnoughAuthors(*new_commit_id))
                } else {
                    info!(
                        "Multiple author verification passed for {}: found multiple authors, {:?}",
                        new_commit_id, authors
                    );
                    Ok(PolicyResult::Ok)
                }
            }
        }
    }
}

fn verify_rebased<G: Git>(
    commits: &[Commit],
    git: &G,
    ref_update: &ReferenceUpdate,
    override_tag_pattern: &Option<String>,
) -> Result<PolicyResult, Box<dyn Error>> {
    match ref_update {
        ReferenceUpdate::Delete { .. } => {
            info!("Rebase verification passed: No checks required for deleting a branch");
            Ok(PolicyResult::Ok)
        }
        ReferenceUpdate::New { new_commit_id, .. } => {
            info!("Rebase verification passed for {}: New branch does not require being rebased for a merge commit", new_commit_id);
            Ok(PolicyResult::Ok)
        }
        ReferenceUpdate::Update {
            old_commit_id,
            new_commit_id,
            ref_name,
        } => {
            let is_merge = git.is_merge_commit(*new_commit_id);
            let is_mainline = git.is_mainline(ref_name)?;
            let new_commit = git.find_commit(*new_commit_id, override_tag_pattern)?;

            if !is_mainline {
                info!(
                    "Rebase verification passed for {}: Not updating a mainline branch",
                    new_commit_id
                );
                Ok(PolicyResult::Ok)
            } else if !is_merge {
                info!(
                    "Rebase verification passed for {}: Not a merge commit",
                    new_commit_id
                );
                Ok(PolicyResult::Ok)
            } else if commits.len() == 0 {
                info!(
                    "Rebase verification passed for {}: No new commits pushed",
                    new_commit_id
                );
                Ok(PolicyResult::Ok)
            } else if !git.is_descendent_of(*new_commit_id, *old_commit_id)? {
                info!(
            "Rebase verification passed for {0}: Commit Id {0} is not a descendent of Commit Id {1}, it is most likely that a force-push has occurred",
            new_commit_id,
            old_commit_id
        );
                Ok(PolicyResult::Ok)
            } else {
                let new_commit_is_rebased = new_commit
                    .parents
                    .iter()
                    .map(|parent_id| {
                        git.is_descendent_of(*parent_id, *old_commit_id)
                            .map(|is_descendent| is_descendent || *parent_id == *old_commit_id)
                    })
                    .collect::<Result<Vec<bool>, _>>()?
                    .iter()
                    .all(|x| *x);

                if new_commit_is_rebased {
                    info!(
                "Rebase verification passed for {}: Branch is up to date with the mainline it's being merged into",
                new_commit_id
            );
                    Ok(PolicyResult::Ok)
                } else {
                    error!("Rebase verification failed for {}: branch must be rebased before it can be merged into the mainline", new_commit_id);
                    Ok(PolicyResult::NotRebased(*new_commit_id))
                }
            }
        }
    }
}

fn verify_email_addresses(
    author_domain: &str,
    committer_domain: &str,
    commits: &[Commit],
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
