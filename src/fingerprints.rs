use crate::git::*;

use std::error::Error;
use log::*;

pub fn read_fingerprints<G: Git>(team_fingerprints_file: &str) -> Result<Vec<String>, Box<dyn Error>>
{
    trace!("Fetching team fingerprints");
    let git = G::new()?;
    let fingerprints_file = git.read_file(team_fingerprints_file)?;
    let fingerprints: Vec<String> = fingerprints_file.split('\n')
        .filter_map( |s| s.split(',').next())
        .map (|s| s.replace(char::is_whitespace, ""))
        .filter(|s| !s.is_empty())
        .collect();
    return Ok(fingerprints);
}