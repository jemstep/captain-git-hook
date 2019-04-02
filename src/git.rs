use git2::Repository;
use std::fs::File;
use std::io::prelude::*;
use std::error::Error;

use crate::config::*;

pub trait Git {
    fn read_file(path: &str) -> Result<String, Box<Error>>;
}

pub struct GitRepo {}

impl Git for GitRepo {
    fn read_file(path: &str) -> Result<String, Box<Error>> {
        let repo = Repository::discover("./")?;

        if let Some(working_dir) = repo.workdir()  {
            let mut read_file = File::open(working_dir.join(path))?;
            let mut current_contents = String::new();
            read_file.read_to_string(&mut current_contents)?;
            Ok(current_contents)
        } else {
            let obj = repo.revparse_single(&format!("HEAD:{}", path))?;
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
}

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


pub fn read_config<G: Git>() -> Result<Config, Box<Error>> {
    let config_str = G::read_file(".capn")?;

    let config = Config::from_toml_string(&config_str)?;
    Ok(config)
}

#[cfg(test)]
mod test {
    use super::*;
    
    pub struct MockGit {}
    impl Git for MockGit {
        fn read_file(_path: &str) -> Result<String, Box<Error>> {
            Ok(String::from(""))
        }
    }

    #[test]
    fn parses_empty_config_to_nones() {
        use crate::config::*;
        
        let config = read_config::<MockGit>().unwrap();
        assert_eq!(
            config,
            Config {
                prepend_branch_name: None,
                example_complex_config: None
            }
        );
    }
}
