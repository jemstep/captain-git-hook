use crate::git::*;

use std::error::Error;
use log::*;

pub struct Fingerprint {
    pub id: String,
    pub name: String,
    pub email: String
}

pub fn read_fingerprints<G: Git>(team_fingerprints_file: &str) -> Result<Vec<Fingerprint>, Box<dyn Error>>
{
    trace!("Fetching team fingerprints");
    let git = G::new()?;
    let fingerprints_file = git.read_file(team_fingerprints_file)?;
    let fingerprints: Vec<Fingerprint> = fingerprints_file.split('\n')
        .filter_map( |s| {
            let mut split_str = s.split(',');
            let fingerprint = match split_str.next(){
                Some(s) => s.replace(char::is_whitespace, ""),
                None => return None
            };
            let name = match split_str.next(){
                Some(s) => s.to_string(),
                None => return None
            };
            let email = match split_str.next(){
                Some(s) => s.to_string(),
                None => return None
            };
            return Some(Fingerprint {id: fingerprint, name: name, email: email});
            }
            )
       
        .collect();
    return Ok(fingerprints);
}