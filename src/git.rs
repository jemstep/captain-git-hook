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
    fn find_commits(&self,from_id: Oid,to_id: Oid) -> Result<Vec<Commit<'_>>, Box<dyn Error>>;
    fn find_unpushed_commits(&self, new_commit_id: Oid) -> Result<Vec<Commit<'_>>, Box<dyn Error>>;
    fn find_commit(&self,commit_id: Oid) -> Result<Commit<'_>, Box<dyn Error>>;
    fn find_commit_fingerprints(&self, team_fingerprint_file: &str, commits: &Vec<Commit<'_>>) -> Result<HashSet<String>, Box<dyn Error>>;
    fn find_common_ancestor(&self, commit1_id: Oid, commit2_id: Oid) -> Result<Oid, Box<dyn Error>>;
    fn pushed(&self,commit_id: Oid) -> Result<bool, Box<dyn Error>>;
    fn not_merge_commit(commit: &Commit<'_>) -> bool;
    fn merge_commit(commit: &Commit<'_>) -> bool;
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
        for commit_id in revwalk {
            let commit = self.repo.find_commit(commit_id?)?;
            Self::debug_commit(&commit);
        }

        Ok(())
    }

    fn find_commit(&self, commit_id: Oid) -> Result<Commit<'_>, Box<dyn Error>> {
        Ok(self.repo.find_commit(commit_id)?)
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
            let fingerprint = team_fingerprints.get(commit_email);
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

    fn find_commits(&self, from_id: Oid, to_id: Oid) -> Result<Vec<Commit<'_>>, Box<dyn Error>> {
        debug!("Find commits between {} to {}", from_id, to_id);

        let commits : Vec<_> = CommitIterator::new(&self.repo, Some(to_id))
        .take_while(|c| c.id() != from_id)
        .collect();

        debug!("Commits found {:#?}",commits);
        Ok(commits)
    }

    fn find_unpushed_commits(&self, new_commit_id: Oid) -> Result<Vec<Commit<'_>>, Box<dyn Error>> {
        debug!("Get unpushed commits from {} ", new_commit_id);

        let commits : Vec<_> = CommitIterator::new(&self.repo, Some(new_commit_id))
        .take_while(|c| {
            match self.pushed(c.id()) {
                Ok(p) => !p,
                Err(_) => true
            }
        })
        .collect();

        debug!("Unpushed Commits found {:#?}",commits);
        Ok(commits)
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

    fn find_common_ancestor(&self, commit1_id: Oid, commit2_id: Oid) -> Result<Oid, Box<dyn Error>> {
        debug!("Find common ancestor for commits {} {}", commit1_id, commit2_id);
        let repo_path = self.repo.path();
        debug!("Repo path {:?}", repo_path);
        let result = Command::new("git")
            .current_dir(repo_path)
            .arg("merge-base")
            .arg(commit1_id.to_string())
            .arg(commit2_id.to_string())
            .output()?;
        debug!("RESULT {:?}", result);
        if !result.status.success() {
            return Err(Box::new(CapnError::new(format!("Call to git merge base failed with status {}", result.status))));
        }

        let encoded = String::from_utf8(result.stdout)?;
        let shas = encoded.split('\n')
            .filter_map(|s| Some(String::from(s)))
            .collect::<Vec<_>>();
        let first_sha = shas.first();
        debug!("Found valid common ancestor : {:?}", first_sha);
        match first_sha {
            Some(f) => return Ok(Oid::from_str(f)?),
            None => return Err(Box::new(CapnError::new(format!("Common ancestors for commits {} {} not found", commit1_id, commit2_id))))
        };
             
    }

    fn not_merge_commit(commit: &Commit<'_>) -> bool {
        let parent_count = commit.parent_count();
        return if parent_count == 1 { true } else { false };
    }

    fn merge_commit(commit: &Commit<'_>) -> bool {
        let parent_count = commit.parent_count();
        return if parent_count > 1 { true } else { false };
    }
   
}

struct CommitIterator<'a> {
    repo: &'a Repository,
    current_commit_id: Option<Oid>
}

impl CommitIterator<'_>  {
    fn new(repo: &Repository,initial_commit_id: Option<Oid> ) -> CommitIterator<'_>  {
        CommitIterator { repo : repo, current_commit_id : initial_commit_id }
    }
}

impl<'a> Iterator for CommitIterator<'a>  {
    type Item = Commit<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let commit = self.current_commit_id.map(|id| {
            let current_commit = self.repo.find_commit(id);
            match current_commit {
                Ok(c) => {
                    let parent_count = c.parent_count();
                    if parent_count == 0 || parent_count > 1 {
                        self.current_commit_id = None;
                    } else {
                        let parent = c.parents().nth(0);
                        self.current_commit_id = match parent {
                            Some(p) => Some(p.id()),
                            _ => None
                        };
                    }
                    return Some(c);
                },
                Err(_) => return None
            }
        });
        return commit.and_then(|c| c);
    }
}