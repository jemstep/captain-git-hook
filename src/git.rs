use git2::Repository;
use std::fs::File;
use std::io::prelude::*;
use std::error::Error;

/// Uses libgit to get the name of your current branch
pub fn get_current_branch() -> Result<String, git2::Error> {
    let git_repo = Repository::discover("./")?;
    let head = git_repo.head()?;
    let head_name =  head.shorthand();
    match head_name {
        Some(name) => Ok(name.to_string()),
        None => Err(git2::Error::from_str("No branch name found"))
    }
}


pub fn read_config() -> Result<String, Box<Error>> {
    let repo = Repository::discover("./")?;

    if let Some(working_dir) = repo.workdir()  {
        let mut read_file = File::open(working_dir.join(".capn"))?;
        let mut current_contents = String::new();
        read_file.read_to_string(&mut current_contents)?;
        Ok(current_contents)
    } else {
        let obj = repo.revparse_single("HEAD:.capn")?;
        if let Some(blob) = obj.as_blob() {
            match String::from_utf8(blob.content().to_vec()) {
                Ok(config_str) => Ok(config_str),
                Err(e) => Err(Box::new(git2::Error::from_str(&format!("Config file is not UTF-8 encoded: {}", e))))
            }
        } else {
            Err(Box::new(git2::Error::from_str("Config is not a blob")))
        }
    }
}
