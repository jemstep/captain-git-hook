use crate::git::*;
use crate::gpg::*;
use crate::fs::*;
use crate::fingerprints::*;


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

pub fn verify_git_commits<G: Git, P: Gpg>(_new_value: &str, team_fingerprints_file: &str, keyserver: &str, recv_keys_par: bool) -> Result<(), Box<dyn Error>> {
    trace!("Executing policy: verify_git_commits");
    let start = Instant::now();

    let fingerprints = read_fingerprints::<G>(team_fingerprints_file)?;
    if recv_keys_par {
        let _result = P::par_receive_keys(&keyserver,&fingerprints)?;
    } else {
        let _result = P::receive_keys(&keyserver,&fingerprints)?;
    }

    let duration = start.elapsed();
    trace!("verify_git_commits completed in: {}s", duration.as_secs());
    Ok(())
}

