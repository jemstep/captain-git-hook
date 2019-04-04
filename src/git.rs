use git2::Repository;
use std::fs::File;
use std::io::prelude::*;
use std::error::Error;

use crate::config::*;

pub trait Git {
    fn read_file(&self, path: &str) -> Result<String, Box<Error>>;
    fn write_git_file(&self, path: &str, contents: &str) -> Result<(), Box<Error>>;
    
    fn current_branch(&self) -> Result<String, Box<Error>>;
    
    fn read_config(&self) -> Result<Config, Box<Error>> {
        let config_str = self.read_file(".capn")?;

        let config = Config::from_toml_string(&config_str)?;
        Ok(config)
    }

}

pub struct LiveGit {
    repo: Repository
}

impl LiveGit {
    pub fn new() -> Result<LiveGit, Box<Error>> {
        let repo = Repository::discover("./")?;
        Ok(LiveGit { repo })
    }
}

impl Git for LiveGit {
    fn read_file(&self, path: &str) -> Result<String, Box<Error>> {
        if let Some(working_dir) = self.repo.workdir()  {
            let mut read_file = File::open(working_dir.join(path))?;
            let mut current_contents = String::new();
            read_file.read_to_string(&mut current_contents)?;
            Ok(current_contents)
        } else {
            let obj = self.repo.revparse_single(&format!("HEAD:{}", path))?;
            if let Some(blob) = obj.as_blob() {
                match String::from_utf8(blob.content().to_vec()) {
                    Ok(config_str) => Ok(config_str),
                    Err(e) => Err(Box::new(git2::Error::from_str(&format!("File is not UTF-8 encoded. {}", e))))
                }
            } else {
                Err(Box::new(git2::Error::from_str("File path does not refer to a file")))
            }
        }
    }

    fn write_git_file(&self, path: &str, contents: &str) -> Result<(), Box<Error>> {
        use std::os::unix::fs::PermissionsExt;

        let dotgit_dir = self.repo.path();
        let mut file = File::create(dotgit_dir.join(path))?;
        file.set_permissions(PermissionsExt::from_mode(0o750))?;
        file.write_all(contents.as_bytes())?;

        Ok(())
    }

    fn current_branch(&self) -> Result<String, Box<Error>> {
        let head = self.repo.head()?;
        let head_name =  head.shorthand();
        match head_name {
            Some(name) => Ok(name.to_string()),
            None => Err(Box::new(git2::Error::from_str("No branch name found")))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    
    pub struct MockGit {}
    impl Git for MockGit {
        fn read_file(&self, _path: &str) -> Result<String, Box<Error>> {
            Ok(String::from(""))
        }
        fn current_branch(&self) -> Result<String, Box<Error>> {
            Ok(String::from("master"))
        }
        fn write_git_file(&self, path: &str, contents: &str) -> Result<(), Box<Error>> {
            Ok(())
        }
    }

    #[test]
    fn parses_empty_config_to_nones() {
        use crate::config::*;
        
        let config = (MockGit{}).read_config().unwrap();
        assert_eq!(
            config,
            Config {
                prepend_branch_name: None,
                verify_git_commits: None,
                example_complex_config: None
            }
        );
    }
}
