use crate::git::*;
use crate::gpg::*;
use crate::fs::*;
use crate::error::CapnError;
use crate::config::VerifyGitCommitsConfig;
use crate::pretty::*;

use git2::{Commit, Oid};
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;
use std::collections::HashSet;
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

    if !G::is_deleted_branch(new_commit_id) {

        let new_commit = git.find_commit(new_commit_id)?;
        let new_branch = G::is_new_branch(old_commit_id);
        let merging = G::merge_commit(&new_commit) && !new_branch;
        let commits = commits_to_verify(&git, old_commit_id, new_commit)?;

        info!("Number of commits to verify {} : ", commits.len());
        for commit in &commits { G::debug_commit(&commit) };
        info!("{}", seperator(""));
        info!("Find fingerprints for commits, and receive associated gpg keys");
        let commit_fingerprints = git.find_commit_fingerprints(&config.team_fingerprints_file, &commits)?;
        if config.recv_keys_par {
            let _result = P::par_receive_keys(&config.keyserver,&commit_fingerprints)?;
        } else {
            let _result = P::receive_keys(&config.keyserver,&commit_fingerprints)?;
        }
        info!("{}", seperator(""));
        if config.verify_email_addresses {
            verify_email_addresses(&config.author_domain, &config.committer_domain, &commits)?;
            info!("{}", seperator(""));
        }
       
        if config.verify_commit_signatures {
            verify_commit_signatures::<G>(&git, &commits)?;
            info!("{}", seperator(""));
        }
        
        if merging && config.verify_different_authors {
            verify_different_authors::<G>(&commits)?;
            info!("{}", seperator(""));
        }

    } else {
        info!("{}", block("DELETE BRANCH detected, not verifying."))
    }

    let duration = start.elapsed();

    info!("verify_git_commits COMPLETED in: {} ms", duration.as_millis());

    Ok(())
    // return Err(Box::new(CapnError::new(format!("Error on verify git commits for testing"))));
}

fn commits_to_verify<'a, G: Git>(git: &'a G, old_commit_id: Oid, new_commit: Commit<'a>) -> Result<Vec<Commit<'a>>, Box<dyn Error>>  {
    let mut commits = Vec::new();
    if G::is_new_branch(old_commit_id) {
        info!("{}", block("NEW BRANCH detected"));
        commits = git.find_unpushed_commits(new_commit.id())?;
    } else if G::merge_commit(&new_commit) {
        info!("{}", block("MERGE detected"));
        match new_commit.parents().nth(1) {
            Some(second_parent) => {
                commits.push(new_commit);
                let common_ancestor_id = git.find_common_ancestor(old_commit_id, second_parent.id())?;
                let mut commits2 = git.find_commits(common_ancestor_id, second_parent.id())?;
                commits.append(&mut commits2);
            },
            None => return Err(Box::new(CapnError::new(format!("Second parent not found for merge commit {}", new_commit.id()))))
        };
    } else {
        commits = git.find_commits(old_commit_id, new_commit.id())?;
    }
    Ok(commits)
}


fn verify_commit_signatures<G: Git>(git: &G, commits: &Vec<Commit<'_>>) -> Result<(), Box<dyn Error>> {
    info!("Verify commit signatures");
    for commit in commits.iter() {
        if G::not_merge_commit(commit) {
            let _fingerprint = git.verify_commit_signature(commit)?;
        }
    }
    Ok(())
}

fn verify_different_authors<G: Git>(commits: &Vec<Commit<'_>>) -> Result<(), Box<dyn Error>> {
    info!("Verify multiple authors");
    let authors : HashSet<_> = commits.iter().filter_map(|c| {
        match c.author().name() {
            Some(n) => Some(n.to_string()),
            _ => None
        }
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
            Some(s) => if s.contains(author_domain) == false {
                    return Err(Box::new(CapnError::new(format!("Author {:?} : Commit {} : Email address {:?} incorrect.",
                        commit.author().name(), commit.id(), commit.author().email()))))
                },
            None => return Err(Box::new(CapnError::new(format!("Author {:?} : Commit {} : No email address.",
                        commit.author().name(), commit.id()))))
        }

        match commit.committer().email(){
            Some(s) => if s.contains(committer_domain) == false {
                     return Err(Box::new(CapnError::new(format!("Committer {:?} : Commit {} : Email address {:?} incorrect.",
                        commit.committer().name(), commit.id(), commit.committer().email()))))
                },
              None => return Err(Box::new(CapnError::new(format!("Committer {:?} : Commit {} : No email address.",
                        commit.committer().name(), commit.id()))))
        }
    }
    Ok(())
}

