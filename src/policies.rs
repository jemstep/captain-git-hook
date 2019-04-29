use crate::git::*;
use crate::gpg::*;
use crate::fs::*;
use crate::error::CapnError;
use crate::config::VerifyGitCommitsConfig;

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
    debug!("Executing policy: verify_git_commits");
    
    let git = G::new()?;
    let start = Instant::now();
    let old_commit_id = Oid::from_str(old_value)?;
    let new_commit_id = Oid::from_str(new_value)?;

    let new_commit = git.find_commit(new_commit_id)?;
    let commits = git.commit_range(old_commit_id, new_commit_id)?;
    let commit_fingerprints = git.find_commit_fingerprints(&config.team_fingerprints_file, &commits)?;

    if config.recv_keys_par {
        let _result = P::par_receive_keys(&config.keyserver,&commit_fingerprints)?;
    } else {
        let _result = P::receive_keys(&config.keyserver,&commit_fingerprints)?;
    }

    verify_email_addresses(&config.author_domain, &config.committer_domain, &commits)?;

    verify_commit_signatures::<G>(&git, &commits)?;

    verify_different_authors::<G>(&new_commit, &commits)?;

    let duration = start.elapsed();

    info!("verify_git_commits completed in: {} ms", duration.as_millis());

    Ok(())
    //return Err(Box::new(CapnError::new(format!("Error on verify git commits for testing"))));
}

fn verify_commit_signatures<G: Git>(git: &G, commits: &Vec<Commit<'_>>) -> Result<(), Box<dyn Error>> {
    debug!("Verify commit signatures");
    for commit in commits.iter() {
        if G::single_commit(commit)? {
            let _fingerprint = git.verify_commit_signature(commit.id())?;
        }
    }
    Ok(())
}

fn verify_different_authors<G: Git>(new_commit: &Commit<'_>, commits: &Vec<Commit<'_>>) -> Result<(), Box<dyn Error>> {
    debug!("Verify different authors");

    let mut authors = HashSet::new();
    if G::merge_commit(&new_commit)? {
        debug!("MERGE commit detected");
        for commit in commits.iter() {
            G::debug_commit(&commit);
            match commit.author().name() {
                Some(n) => authors.insert(n.to_string()),
                None => return Err(Box::new(CapnError::new(format!("No author name found on commit"))))
            };
        }
        debug!("Author set: {:#?}", authors);
        if authors.len() <= 1 {
            return Err(Box::new(CapnError::new(format!("Only one author present"))))
        }
    }
    
    Ok(())
}

fn verify_email_addresses(author_domain: &str,committer_domain: &str, commits: &Vec<Commit<'_>>) -> Result<(), Box<dyn Error>> {
    debug!("Verify email addresses");
    
    for commit in commits.iter() {
        match commit.author().email(){
            Some(s) => if s.contains(author_domain) == false {
                    return Err(Box::new(CapnError::new(format!("Author email address incorrect"))))
                },
            None => return Err(Box::new(CapnError::new(format!("Author email address incorrect"))))
        }

        match commit.committer().email(){
            Some(s) => if s.contains(committer_domain) == false {
                    return Err(Box::new(CapnError::new(format!("Committer email address incorrect"))))
                },
            None => return Err(Box::new(CapnError::new(format!("Committer email address incorrect"))))
        }
    }
    Ok(())
}

