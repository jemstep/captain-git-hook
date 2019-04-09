use crate::git::*;
use crate::gpg::*;

use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::io::prelude::*;
use std::time::Instant;
use log::*;

pub fn prepend_branch_name(commit_file: PathBuf) -> Result<(), Box<Error>> {
    debug!("Executing policy: prepend_branch_name");
    
    let git = LiveGit::new()?;
    let branch = git.current_branch()?;
    Ok(prepend_string_to_file(branch, commit_file)?)
}

fn prepend_string_to_file(s: String, filename: PathBuf) -> Result<(), std::io::Error> {
    // It turns out that prepending a string to a file is not an
    // obvious action. You can only write to the end of a file :(
    //
    // The solution is to read the existing contents, then write a new
    // file starting with the branch name, and then writing the rest
    // of the file.

    let mut read_file = File::open(&filename)?;
    let mut current_contents = String::new();
    read_file.read_to_string(&mut current_contents)?;

    let mut write_file = File::create(&filename)?;

    writeln!(write_file, "{}:", s)?;
    write!(write_file, "{}", current_contents)
}

pub fn verify_git_commits(new_value: &str, team_fingerprints_file: &str, keyserver: &str) -> Result<(), Box<Error>> {
    debug!("Executing policy: verify_git_commits");
    
    let start = Instant::now();
    
    trace!("Fetching team fingerprints");
    let git = LiveGit::new()?;
    let fingerprints_file = git.read_file(team_fingerprints_file)?;
    let fingerprints: Vec<String> = fingerprints_file.split('\n')
        .filter_map( |s| s.split(',').next())
        .map (|s| s.replace(char::is_whitespace, ""))
        .filter(|s| !s.is_empty())
        .collect();

    trace!("Receive latest keys from key server");
    (LiveGpg{}).receive_keys(keyserver,&fingerprints)?;
    let duration = start.elapsed();
    trace!("verify_git_commits completed in: {}ms", duration.as_millis());
    Ok(())
}

