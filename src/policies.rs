use crate::git::*;
use crate::gpg::*;
use crate::fs::*;
use crate::error::CapnError;
use crate::config::VerifyGitCommitsConfig;
use crate::pretty::*;
use crate::fingerprints::*;

use git2::{Commit, Oid};
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;
use std::collections::{HashMap, HashSet};
use log::*;



pub fn prepend_branch_name<F: Fs, G: Git>(commit_file: PathBuf) -> Result<(), Box<dyn Error>> {
    debug!("Executing policy: prepend_branch_name");
    
    let git = G::new()?;
    let branch = git.current_branch()?;
    Ok(F::prepend_string_to_file(branch, commit_file)?)
}

pub fn verify_git_commits<G: Git, P: Gpg>(config: &VerifyGitCommitsConfig, old_value: &str, new_value: &str,_ref_name: &str) -> Result<(), Box<dyn Error>> {
    info!("{}", seperator("verify_git_commits STARTED"));
    let git = G::new()?;
    let start = Instant::now();
    let old_commit_id = Oid::from_str(old_value)?;
    let new_commit_id = Oid::from_str(new_value)?;

    if G::is_deleted_branch(new_commit_id) {
        info!("{}", block("DELETE BRANCH detected, no commits to verify."))
    } else if git.is_tag(new_commit_id) {
        info!("{}", block("TAG detected, no commits to verify."))
    } else {
        let new_commit = git.find_commit(new_commit_id)?;
        let new_branch = G::is_new_branch(old_commit_id);
        let merging = G::merge_commit(&new_commit) && !new_branch;
        let commits = commits_to_verify(&git, old_commit_id, new_commit)?;

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
            verify_email_addresses(&config.author_domain, &config.committer_domain, &commits)?;
            info!("{}", seperator(""));
        }
       
        if config.verify_commit_signatures {
            verify_commit_signatures::<G>(&git, &commits, &commit_fingerprints)?;
            info!("{}", seperator(""));
        }
        
        if merging && config.verify_different_authors {
            verify_different_authors::<G>(&commits)?;
            info!("{}", seperator(""));
        }
    }

    let duration = start.elapsed();

    info!("verify_git_commits COMPLETED in: {} ms", duration.as_millis());

    Ok(())
}

fn commits_to_verify<'a, G: Git>(git: &'a G, old_commit_id: Oid, new_commit: Commit<'a>) -> Result<Vec<Commit<'a>>, Box<dyn Error>>  {
    if G::is_new_branch(old_commit_id) {
        info!("{}", block("NEW BRANCH detected"));
        git.find_unpushed_commits(new_commit.id())
    } else {
        git.find_commits(old_commit_id, new_commit.id())
    }
}


fn verify_commit_signatures<G: Git>(git: &G, commits: &Vec<Commit<'_>>, fingerprints: &HashMap<String, Fingerprint>) -> Result<(), Box<dyn Error>> {
    info!("Verify commit signatures");
    for commit in commits.iter() {
        if G::is_identical_tree_to_any_parent(commit) {
            debug!("{}: verified identical to one of its parents, no signature required", commit.id());
        } else if git.is_trivial_merge_commit(commit) {
            debug!("{}: verified to be a trivial merge of its parents, no signature required", commit.id());
        } else {
            match git.verify_commit_signature(commit, fingerprints) {
                Ok(_) => {
                    debug!("{}: verified with a valid signature", commit.id());
                },
                Err(err) => {
                    debug!("{}: unverified, requies a valid signature", commit.id());
                    return Err(err);
                }
            }
        }
    }
    Ok(())
}

fn verify_different_authors<G: Git>(commits: &Vec<Commit<'_>>) -> Result<(), Box<dyn Error>> {
    info!("Verify multiple authors");
    let authors : HashSet<_> = commits.iter().filter_map(|c| {
        c.author().email().map(|e| e.to_string())
    }).collect();
    debug!("Author set: {:#?}", authors);
    if authors.len() <= 1 {
        return Err(Box::new(CapnError::new(format!("None or only one author present"))))
    }
    Ok(())
}

fn verify_email_addresses(author_domain: &str,committer_domain: &str, commits: &Vec<Commit<'_>>) -> Result<(), Box<dyn Error>> {
    info!("Verify email addresses");
    for commit in commits.iter() {
        debug!("Verify author, committer email addresses for commit {}", commit.id());
        match commit.author().email(){
            Some(s) => if !s.ends_with(&format!("@{}", author_domain)) {
                return Err(Box::new(CapnError::new(format!("Author {:?} : Commit {} : Email address {:?} incorrect.",
                                                           commit.author().name(), commit.id(), commit.author().email()))))
            },
            None => return Err(Box::new(CapnError::new(format!("Author {:?} : Commit {} : No email address.",
                                                               commit.author().name(), commit.id()))))
        }

        match commit.committer().email(){
            Some(s) => if !s.ends_with(&format!("@{}", committer_domain)) {
                return Err(Box::new(CapnError::new(format!("Committer {:?} : Commit {} : Email address {:?} incorrect.",
                                                           commit.committer().name(), commit.id(), commit.committer().email()))))
            },
            None => return Err(Box::new(CapnError::new(format!("Committer {:?} : Commit {} : No email address.",
                                                               commit.committer().name(), commit.id()))))
        }
    }
    Ok(())
}

