use crate::git::*;
use crate::gpg::*;
use crate::fs::*;
use crate::fingerprints::*;
use crate::error::CapnError;
use crate::config::VerifyGitCommitsConfig;
use git2::{Commit};


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

pub fn verify_git_commits<G: Git, P: Gpg>(config: &VerifyGitCommitsConfig, old_value: &str, new_value: &str,_ref_name: &str, team_fingerprints_file: &str) -> Result<(), Box<dyn Error>> {
    trace!("Executing policy: verify_git_commits");
    let git = G::new()?;

    let start = Instant::now();

    let _fingerprints = read_fingerprints::<G>(team_fingerprints_file)?;

    let ids = git.commit_range(old_value, new_value)?;
    trace!("PRINT RANGE COMMITS");
    for id in ids {
        let commit = git.find_commit(&id)?;
        G::print_commit(&commit);
        verify_email_address_domain(&config.author_domain, &config.committer_domain, &commit)?;
    }


    //update relevant author keys
    // if recv_keys_par {
    //     let _result = P::par_receive_keys(&keyserver,&fingerprints)?;
    // } else {
    //     let _result = P::receive_keys(&keyserver,&fingerprints)?;
    // }

    //ensure merge commits are only on develop/master

    //ensure merge and content author are different

    //ensure individual commits are gpg signed

    let duration = start.elapsed();
    trace!("verify_git_commits completed in: {}s", duration.as_secs());
    //Ok(())
    return Err(Box::new(CapnError::new(format!("Error on verify git commits for testing"))));
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

