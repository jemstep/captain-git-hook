use crate::git::*;
use crate::gpg::*;
use crate::fs::*;
use crate::error::CapnError;
use crate::config::VerifyGitCommitsConfig;

use git2::{Commit, Oid};
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;
use log::*;

pub fn prepend_branch_name<F: Fs, G: Git>(commit_file: PathBuf) -> Result<(), Box<dyn Error>> {
    trace!("Executing policy: prepend_branch_name");
    
    let git = G::new()?;
    let branch = git.current_branch()?;
    Ok(F::prepend_string_to_file(branch, commit_file)?)
}

pub fn verify_git_commits<G: Git, P: Gpg>(config: &VerifyGitCommitsConfig, old_value: &str, new_value: &str,_ref_name: &str) -> Result<(), Box<dyn Error>> {
    trace!("Executing policy: verify_git_commits");
    
    let git = G::new()?;
    let start = Instant::now();

    let commits = git.commit_range(Oid::from_str(old_value)?, Oid::from_str(new_value)?)?;

    let commit_fingerprints = G::find_commit_fingerprints(&config.team_fingerprints_file, &commits)?;

    if config.recv_keys_par {
        let _result = P::par_receive_keys(&config.keyserver,&commit_fingerprints)?;
    } else {
        let _result = P::receive_keys(&config.keyserver,&commit_fingerprints)?;
    }

    for commit in commits.iter() {
        verify_email_address_domain(&config.author_domain, &config.committer_domain, &commit)?;
        let _fingerprint = git.verify_commit_signature(commit.id())?;
    }

    let duration = start.elapsed();
    trace!("verify_git_commits completed in: {} ms", duration.as_millis());
    Ok(())
    //return Err(Box::new(CapnError::new(format!("Error on verify git commits for testing"))));
}

fn verify_email_address_domain(author_domain: &str,committer_domain: &str, commit: &Commit<'_>) -> Result<(), Box<dyn Error>> {
    trace!("Verify email address domain");
    match commit.author().email(){
        Some(s) => if s.contains(author_domain) == false {
                return Err(Box::new(CapnError::new(format!("Author email address incorrect"))))
            } else {
                println!("OK")
            },
        None => return Err(Box::new(CapnError::new(format!("Author email address incorrect"))))
    }

    match commit.committer().email(){
        Some(s) => if s.contains(committer_domain) == false {
                return Err(Box::new(CapnError::new(format!("Committer email address incorrect"))))
            } else {
                println!("OK")
            },
        None => return Err(Box::new(CapnError::new(format!("Committer email address incorrect"))))
    }
        
    Ok(())
}

