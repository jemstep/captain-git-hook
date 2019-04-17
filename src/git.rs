use git2::{Repository, Commit, Oid};
use std::fs::File;
use std::io::prelude::*;
use std::error::Error;

use crate::config::*;
use log::*;

pub trait Git: Sized {
    fn new() -> Result<Self, Box<dyn Error>>;
    fn read_file(&self, path: &str) -> Result<String, Box<dyn Error>>;
    fn write_git_file(&self, path: &str, file_mode: u32, contents: &str) -> Result<(), Box<dyn Error>>;
    
    fn current_branch(&self) -> Result<String, Box<dyn Error>>;

    fn log(&self) -> Result<(), Box<dyn Error>>;

    fn commit_range(&self,from_id: &str,to_id: &str) -> Result<Vec<String>, Box<dyn Error>>;

    fn find_commit(&self,commit_id: &str) -> Result<Commit<'_>, Box<dyn Error>>;
    
    fn read_config(&self) -> Result<Config, Box<dyn Error>> {
        let config_str = self.read_file(".capn")?;

        let config = Config::from_toml_string(&config_str)?;
        Ok(config)
    }

    fn print_commit(commit: &Commit<'_>) {
        println!("commit {}", commit.id());

        if commit.parents().len() > 1 {
            print!("Merge:");
            for id in commit.parent_ids() {
                print!(" {:.8}", id);
            }
            println!("");
        }

        let author = commit.author();
        println!("Author: {}", author);
        let committer = commit.committer();
        println!("Committer: {}", committer);
        println!("");

        for line in String::from_utf8_lossy(commit.message_bytes()).lines() {
            println!("    {}", line);
        }
        println!("");
    }

}

pub struct LiveGit {
    repo: Repository
}

impl Git for LiveGit {
    fn new() -> Result<Self, Box<dyn Error>> {
        let repo = Repository::discover("./")?;
        Ok(LiveGit { repo })
    }
    
    fn read_file(&self, path: &str) -> Result<String, Box<dyn Error>> {
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

    #[cfg(windows)]
    fn write_git_file(&self, path: &str, _file_mode: u32, contents: &str) -> Result<(), Box<dyn Error>> {
        let dotgit_dir = self.repo.path();
        let mut file = File::create(dotgit_dir.join(path))?;
        file.write_all(contents.as_bytes())?;

        Ok(())
    }

    #[cfg(unix)]
    fn write_git_file(&self, path: &str, file_mode: u32, contents: &str) -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::PermissionsExt;

        let dotgit_dir = self.repo.path();
        let mut file = File::create(dotgit_dir.join(path))?;
        file.set_permissions(PermissionsExt::from_mode(file_mode))?;

        file.write_all(contents.as_bytes())?;

        Ok(())
    }

    fn current_branch(&self) -> Result<String, Box<dyn Error>> {
        let head = self.repo.head()?;
        let head_name =  head.shorthand();
        match head_name {
            Some(name) => Ok(name.to_string()),
            None => Err(Box::new(git2::Error::from_str("No branch name found")))
        }
    }

    fn log(&self) -> Result<(), Box<dyn Error>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        for id in revwalk {
            let id = id?;
            let commit = self.repo.find_commit(id)?;
            Self::print_commit(&commit);
        }
        Ok(())
    }

    fn find_commit(&self, commit_id: &str) -> Result<Commit<'_>, Box<dyn Error>> {
        let new_commit = self.repo.find_commit(Oid::from_str(commit_id)?)?;
        Ok(new_commit)                
    }

    fn commit_range(&self,from_id: &str,to_id: &str) -> Result<Vec<String>, Box<dyn Error>> {
        trace!("Get commit range from {} to {}", from_id, to_id);
        let mut v = Vec::new();
        v.push(to_id.to_string());
        let new_commit = self.repo.find_commit(Oid::from_str(to_id)?)?;
        let mut current_id = to_id.to_string();
        let mut parent_count = new_commit.parent_count();
        while current_id != from_id && parent_count > 0 {
            let current_commit = self.repo.find_commit(Oid::from_str(&current_id)?)?;
            for parent in current_commit.parents() {
                current_id = Oid::to_string(&parent.id());
                let parent_commit = self.repo.find_commit(parent.id())?;
                parent_count = parent_commit.parent_count();
                if current_id != from_id{
                    v.push(current_id.to_string());
                }
            }      
        }
        Ok(v)         
    }
   
}

#[cfg(test)]
mod test {
    use super::*;
    
    pub struct MockGit;
    impl Git for MockGit {
        fn new() -> Result<Self, Box<dyn Error>> {
            Ok(MockGit)
        }
        fn read_file(&self, _path: &str) -> Result<String, Box<dyn Error>> {
            Ok(String::from(""))
        }
        fn current_branch(&self) -> Result<String, Box<dyn Error>> {
            Ok(String::from("master"))
        }
        fn write_git_file(&self, _path: &str, _file_mode: u32, _contents: &str) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
       
        fn log(&self) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        fn find_commit(&self,commit_id: &str) -> Result<Commit<'_>, Box<dyn Error>> {
            //figure out how to create Commit
        }

        fn commit_range(&self,_from_id: &str,_to_id: &str) -> Result<Vec<String>, Box<dyn Error>> {
            let mut v = Vec::new();
            v.push("".to_string());
            Ok(v)         
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
