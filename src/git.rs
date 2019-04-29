use git2::{Repository, Commit, Oid};
use std::fs::File;
use std::io::prelude::*;
use crate::fingerprints::Fingerprint;
use std::error::Error;
use std::process::*;
use crate::error::CapnError;
use std::str;
use std::collections::HashSet;


use crate::config::*;
use log::*;

pub trait Git: Sized {
    fn new() -> Result<Self, Box<dyn Error>>;
    fn read_file(&self, path: &str) -> Result<String, Box<dyn Error>>;
    fn write_git_file(&self, path: &str, file_mode: u32, contents: &str) -> Result<(), Box<dyn Error>>;
    fn current_branch(&self) -> Result<String, Box<dyn Error>>;
    fn log(&self) -> Result<(), Box<dyn Error>>;
    fn commit_range(&self,from_id: Oid,to_id: Oid) -> Result<Vec<Commit<'_>>, Box<dyn Error>>;
    fn find_commit(&self,commit_id: Oid) -> Result<Commit<'_>, Box<dyn Error>>;
    fn find_commit_fingerprints(&self, team_fingerprint_file: &str, commits: &Vec<Commit<'_>>) -> Result<HashSet<String>, Box<dyn Error>>;
    fn pushed(&self,commit_id: Oid) -> Result<bool, Box<dyn Error>>;
    fn single_commit(commit: &Commit<'_>) -> Result<bool, Box<dyn Error>>;
    fn merge_commit(commit: &Commit<'_>) -> Result<bool, Box<dyn Error>>;
    fn verify_commit_signature(&self,commit_id: Oid) -> Result<String, Box<dyn Error>>;
    
    fn read_config(&self) -> Result<Config, Box<dyn Error>> {
        let config_str = self.read_file(".capn")?;
        let config = Config::from_toml_string(&config_str)?;
        Ok(config)
    }

    fn debug_commit(commit: &Commit<'_>) {
        debug!("commit {}", commit.id());
        if commit.parents().len() > 1 {
            debug!("Merge:");
            for id in commit.parent_ids() {
                debug!(" {:.8}", id);
            }
            debug!("");
        }
        let author = commit.author();
        debug!("Author: {}", author);
        let committer = commit.committer();
        debug!("Committer: {}", committer);
        debug!("");
        for line in String::from_utf8_lossy(commit.message_bytes()).lines() {
            debug!("    {}", line);
        }
        debug!("");
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
            Self::debug_commit(&commit);
        }
        Ok(())
    }

    fn find_commit(&self, commit_id: Oid) -> Result<Commit<'_>, Box<dyn Error>> {
        let new_commit = self.repo.find_commit(commit_id)?;
        Ok(new_commit)                
    }

    fn find_commit_fingerprints(&self, team_fingerprint_file: &str, commits: &Vec<Commit<'_>>) -> Result<HashSet<String>, Box<dyn Error>> {

        let team_fingerprints = Fingerprint::read_fingerprints::<LiveGit>(self, team_fingerprint_file)?;

        let mut commit_fingerprints = HashSet::new();

        for commit in commits.iter() {
            let author = commit.author();
            let commit_email = match author.email() {
                Some(e) => e,
                None => return Err(Box::new(CapnError::new(format!("Email on commit not found"))))
            };
            let fingerprint = team_fingerprints.iter().find(|f| f.email == commit_email);
            match fingerprint{
                Some(f) => commit_fingerprints.insert(f.id.to_string()),
                None => return Err(Box::new(CapnError::new(format!("Team fingerprint not found"))))
            };
        }
        Ok(commit_fingerprints)
    }

    fn pushed(&self, commit_id: Oid) -> Result<bool, Box<dyn Error>> {
        debug!("Check if commit {} has already been pushed", commit_id);

        let repo_path = self.repo.path();
        debug!("Repo path {:?}", repo_path);
        let result = Command::new("git")
            .current_dir(repo_path)
            .arg("branch")
            .arg("--contains")
            .arg(commit_id.to_string())
            .output()?;
        debug!("RESULT {:?}", result);
        if !result.status.success() {
            return Err(Box::new(CapnError::new(format!("Call to git branch contains failed for commit {} with status {}", commit_id,result.status))));
        }

        let output = String::from_utf8(result.stdout)?;
        match output.trim() {
            "" => return Ok(false),
            _ => return Ok(true)
        };                
    }

    fn commit_range(&self, from_id: Oid, to_id: Oid) -> Result<Vec<Commit<'_>>, Box<dyn Error>> {
        debug!("Get commit range from {} to {}", from_id, to_id);

        let mut v = Vec::new();
        let new_commit = self.repo.find_commit(to_id)?;
        let merging = Self::merge_commit(&new_commit)?;
        let mut current_id = to_id;
        let mut pushed = false;
        v.push(new_commit);
        while current_id != from_id {
            if pushed == true { break; }
            let current_commit = self.repo.find_commit(current_id)?;
            for parent in current_commit.parents() {
                current_id = parent.id();
                let parent_commit = self.repo.find_commit(parent.id())?;
                pushed = self.pushed(current_id)?;
                debug!("Commit {} already pushed? {}", current_id, pushed);
                if current_id != from_id && (!pushed || merging)  {
                    v.push(parent_commit);
                }
            }      
        }
        debug!("Commits found {:#?}",v);
        Ok(v)         
    }

    fn verify_commit_signature(&self, commit_id: Oid) -> Result<String, Box<dyn Error>> {
        debug!("Verify commit {}", commit_id);
        let repo_path = self.repo.path();
        debug!("Repo path {:?}", repo_path);
        let result = Command::new("git")
            .current_dir(repo_path)
            .arg("verify-commit")
            .arg("--raw")
            .arg(commit_id.to_string())
            .output()?;
        debug!("RESULT {:?}", result);
        if !result.status.success() {
            return Err(Box::new(CapnError::new(format!("Call to git verify failed for commit {} with status {}", commit_id,result.status))));
        }

        let encoded = String::from_utf8(result.stderr)?;
        let fingerprints = encoded.split('\n')
            .filter(|s| s.contains("VALIDSIG"))
            .filter_map(|s| s.split(' ').nth(2).map(String::from))
            .collect::<Vec<_>>();
        let first = fingerprints.first();
        debug!("Found valid fingerprint from commit signature {:?}", first);
        match first {
            Some(f) => return Ok(f.to_string()),
            None => return Err(Box::new(CapnError::new(format!("Valid fingerprint for commit {} not found", commit_id))))
        };
             
    }

    fn single_commit(commit: &Commit<'_>) -> Result<bool, Box<dyn Error>> {
        let parent_count = commit.parent_count();
        return if parent_count == 1 { Ok(true) } else { Ok(false) };
    }

    fn merge_commit(commit: &Commit<'_>) -> Result<bool, Box<dyn Error>> {
         let parent_count = commit.parent_count();
        return if parent_count > 1 { Ok(true) } else { Ok(false) };
    }
   
}
