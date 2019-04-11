use crate::git::*;
use crate::gpg::*;
use crate::fs::*;

use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;
use log::*;

pub fn prepend_branch_name<F: Fs, G: Git>(commit_file: PathBuf) -> Result<(), Box<dyn Error>> {
    debug!("Executing policy: prepend_branch_name");
    
    let git = G::new()?;
    let branch = git.current_branch()?;
    Ok(F::prepend_string_to_file(branch, commit_file)?)
}

pub fn verify_git_commits<G: Git, P: Gpg>(_new_value: &str, team_fingerprints_file: &str, keyserver: &str) -> Result<(), Box<dyn Error>> {
    debug!("Executing policy: verify_git_commits");
    
    let start = Instant::now();
    
    trace!("Fetching team fingerprints");
    let git = G::new()?;
    let fingerprints_file = git.read_file(team_fingerprints_file)?;
    let fingerprints: Vec<String> = fingerprints_file.split('\n')
        .filter_map( |s| s.split(',').next())
        .map (|s| s.replace(char::is_whitespace, ""))
        .filter(|s| !s.is_empty())
        .collect();

    trace!("Receive latest keys from key server");
    P::receive_keys(keyserver,&fingerprints)?;
    let duration = start.elapsed();
    trace!("verify_git_commits completed in: {}ms", duration.as_millis());
    Ok(())
}

