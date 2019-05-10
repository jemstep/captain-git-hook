use crate::git::*;

use std::error::Error;
use log::*;
use std::collections::HashMap;

pub struct Fingerprint {
    pub id: String,
    pub name: String,
    pub email: String
}

impl Fingerprint {
    pub fn read_fingerprints<G: Git>(git: &G, team_fingerprints_file: &str) -> Result<HashMap<String, Fingerprint>, Box<dyn Error>>
    {
        debug!("Fetching team fingerprints");
        let fingerprints_file = git.read_file(team_fingerprints_file)?;
        let fingerprints: HashMap<String, Fingerprint> = fingerprints_file.split('\n').filter_map( |l| {
            let line: Vec<&str> = l.split(',').collect();
            match &line[..] {
                [fingerprint, name, email] => {
                    let fingerprint = fingerprint.replace(char::is_whitespace, "");
                    return Some((email.to_string(), Fingerprint {id: fingerprint, name: name.to_string(), email: email.to_string()}));
                },
                _ => return None
            }            
        }).collect();
        return Ok(fingerprints);
    }
}