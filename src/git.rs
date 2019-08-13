use crate::error::CapnError;
use crate::fingerprints::Fingerprint;
use git2::{Commit, Oid, Repository};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::process::*;
use std::str;

use crate::config::*;
use log::*;

pub struct VerificationCommit {
    pub id: String,
    pub committer_email: Option<String>,
    pub is_identical_tree: bool,
    pub valid_signature: bool,
    pub fingerprint: Option<String>,
}

pub trait Git: Sized {
    fn new() -> Result<Self, Box<dyn Error>>;
    fn read_file(&self, path: &str) -> Result<String, Box<dyn Error>>;
    fn write_git_file(
        &self,
        path: &str,
        file_mode: u32,
        contents: &str,
    ) -> Result<(), Box<dyn Error>>;
    fn current_branch(&self) -> Result<String, Box<dyn Error>>;
    fn find_commits(&self, from_id: Oid, to_id: Oid) -> Result<Vec<Commit<'_>>, Box<dyn Error>>;
    fn is_tag(&self, id: Oid) -> bool;
    fn find_unpushed_commits(&self, new_commit_id: Oid) -> Result<Vec<Commit<'_>>, Box<dyn Error>>;
    fn find_commit(&self, commit_id: Oid) -> Result<Commit<'_>, Box<dyn Error>>;
    fn find_commit_fingerprints(
        &self,
        team_fingerprint_file: &str,
        commits: &Vec<Commit<'_>>,
    ) -> Result<HashMap<String, Fingerprint>, Box<dyn Error>>;
    fn merge_commit(new_commit: &Commit<'_>) -> bool;
    fn is_identical_tree_to_any_parent(commit: &Commit<'_>) -> bool;
    fn is_trivial_merge_commit(&self, commit: &Commit<'_>) -> Result<bool, Box<dyn Error>>;
    fn is_head(&self, ref_name: &str) -> Result<bool, Box<dyn Error>>;
    fn path(&self) -> &std::path::Path;
    fn verify_commit_signature(
        path: &std::path::Path,
        commit: &VerificationCommit,
    ) -> Result<bool, Box<dyn Error>>;
    fn read_config(&self) -> Result<Config, Box<dyn Error>> {
        let config_str = self.read_file(".capn")?;
        let config = Config::from_toml_string(&config_str)?;
        Ok(config)
    }

    fn debug_commit(commit: &Commit<'_>) {
        debug!(
            "CommitId: {}, Parent(s): {:?}, Author: {}, Committer: {}, Message: {}",
            commit.id(),
            commit
                .parent_ids()
                .fold("".to_string(), |acc, next| if acc.is_empty() {
                    next.to_string()
                } else {
                    format!("{},{}", acc, next)
                }),
            commit.author(),
            commit.committer(),
            commit.summary().unwrap_or("")
        );
    }
}

pub struct LiveGit {
    repo: Repository,
}

impl Git for LiveGit {
    fn new() -> Result<Self, Box<dyn Error>> {
        let repo = Repository::discover("./")?;
        Ok(LiveGit { repo })
    }

    fn path(&self) -> &std::path::Path {
        self.repo.path()
    }

    fn read_file(&self, path: &str) -> Result<String, Box<dyn Error>> {
        if let Some(working_dir) = self.repo.workdir() {
            let mut read_file = File::open(working_dir.join(path))?;
            let mut current_contents = String::new();
            read_file.read_to_string(&mut current_contents)?;
            Ok(current_contents)
        } else {
            let obj = self.repo.revparse_single(&format!("HEAD:{}", path))?;
            if let Some(blob) = obj.as_blob() {
                match String::from_utf8(blob.content().to_vec()) {
                    Ok(config_str) => Ok(config_str),
                    Err(e) => Err(Box::new(git2::Error::from_str(&format!(
                        "File is not UTF-8 encoded. {}",
                        e
                    )))),
                }
            } else {
                Err(Box::new(git2::Error::from_str(
                    "File path does not refer to a file",
                )))
            }
        }
    }

    #[cfg(windows)]
    fn write_git_file(
        &self,
        path: &str,
        _file_mode: u32,
        contents: &str,
    ) -> Result<(), Box<dyn Error>> {
        let dotgit_dir = self.repo.path();
        let mut file = File::create(dotgit_dir.join(path))?;
        file.write_all(contents.as_bytes())?;

        Ok(())
    }

    #[cfg(unix)]
    fn write_git_file(
        &self,
        path: &str,
        file_mode: u32,
        contents: &str,
    ) -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::PermissionsExt;

        let dotgit_dir = self.repo.path();
        let mut file = File::create(dotgit_dir.join(path))?;
        file.set_permissions(PermissionsExt::from_mode(file_mode))?;

        file.write_all(contents.as_bytes())?;

        Ok(())
    }

    fn current_branch(&self) -> Result<String, Box<dyn Error>> {
        let head = self.repo.head()?;
        let head_name = head.shorthand();
        match head_name {
            Some(name) => Ok(name.to_string()),
            None => Err(Box::new(git2::Error::from_str("No branch name found"))),
        }
    }

    fn find_commit(&self, commit_id: Oid) -> Result<Commit<'_>, Box<dyn Error>> {
        Ok(self.repo.find_commit(commit_id)?)
    }

    fn find_commit_fingerprints(
        &self,
        team_fingerprint_file: &str,
        commits: &Vec<Commit<'_>>,
    ) -> Result<HashMap<String, Fingerprint>, Box<dyn Error>> {
        let mut team_fingerprints =
            Fingerprint::read_fingerprints::<LiveGit>(self, team_fingerprint_file)?;

        let commit_emails = commits
            .iter()
            .filter_map(|c| c.committer().email().map(|email| email.to_string()))
            .collect::<HashSet<String>>();

        team_fingerprints.retain(|email, _| commit_emails.contains(email));
        Ok(team_fingerprints)
    }

    fn find_commits(&self, from_id: Oid, to_id: Oid) -> Result<Vec<Commit<'_>>, Box<dyn Error>> {
        debug!("Find commits between {} to {}", from_id, to_id);

        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(to_id)?;
        revwalk.hide(from_id)?;
        revwalk.hide_head()?;

        let commits = revwalk
            .into_iter()
            .map(|id| id.and_then(|id| self.repo.find_commit(id)))
            .collect::<Result<Vec<_>, git2::Error>>()?;

        Ok(commits)
    }

    fn find_unpushed_commits(&self, new_commit_id: Oid) -> Result<Vec<Commit<'_>>, Box<dyn Error>> {
        debug!("Get unpushed commits from {} ", new_commit_id);

        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(new_commit_id)?;
        revwalk.hide_head()?;

        let commits = revwalk
            .into_iter()
            .map(|id| id.and_then(|id| self.repo.find_commit(id)))
            .collect::<Result<Vec<_>, git2::Error>>()?;

        Ok(commits)
    }

    fn verify_commit_signature(
        path: &std::path::Path,
        commit: &VerificationCommit,
    ) -> Result<bool, Box<dyn Error>> {
        let commit_id = &commit.id;

        let committer_email = match &commit.committer_email {
            Some(email) => email,
            None => {
                debug!(
                    "Commit {} does not have a valid committer: no email address",
                    commit_id
                );
                return Ok(false);
            }
        };
        let expected_fingerprint = match &commit.fingerprint {
            Some(f) => f,
            None => {
                debug!(
                    "Did not find GPG key for commit {}, committer {}",
                    commit_id, committer_email
                );
                return Ok(false);
            }
        };

        let result = Command::new("git")
            .current_dir(path)
            .arg("verify-commit")
            .arg("--raw")
            .arg(commit_id.to_string())
            .output()?;
        debug!(
            "Result from calling git verify-commit on {}: {:?}",
            commit_id, result
        );

        let encoded = String::from_utf8(result.stderr)?;

        let valid = encoded
            .split('\n')
            .any(|s| s.contains(&format!("VALIDSIG {}", expected_fingerprint)));

        if valid {
            debug!("Commit {} was signed with a valid signature", commit_id);
            Ok(true)
        } else {
            debug!("Commit {} was not signed with a valid signature", commit_id);
            Ok(false)
        }
    }

    fn merge_commit(new_commit: &Commit<'_>) -> bool {
        let parent_count = new_commit.parent_count();
        return if parent_count > 1 { true } else { false };
    }

    fn is_identical_tree_to_any_parent(commit: &Commit<'_>) -> bool {
        let tree_id = commit.tree_id();
        commit.parents().any(|p| p.tree_id() == tree_id)
    }

    fn is_trivial_merge_commit(&self, commit: &Commit<'_>) -> Result<bool, Box<dyn Error>> {
        use git2::MergeOptions;

        let temp_repo = TempRepo::new(commit.id())?;
        match &commit.parents().collect::<Vec<_>>()[..] {
            [a, b] => {
                let expected_tree_id = commit.tree_id();
                let reproduced_tree_id = self
                    .repo
                    .merge_commits(&a, &b, Some(MergeOptions::new().fail_on_conflict(true)))
                    .and_then(|mut index| index.write_tree_to(&temp_repo.repo));
                let matches = reproduced_tree_id
                    .as_ref()
                    .map(|id| *id == expected_tree_id)
                    .unwrap_or(false);

                Ok(matches)
            }
            _ => Ok(false),
        }
    }

    fn is_head(&self, ref_name: &str) -> Result<bool, Box<dyn Error>> {
        let head = self.repo.head()?;
        Ok(Some(ref_name) == head.name())
    }

    fn is_tag(&self, id: Oid) -> bool {
        match self.repo.find_tag(id) {
            Ok(_) => true,
            _ => false,
        }
    }
}

struct TempRepo {
    repo: Repository,
}

impl TempRepo {
    fn new(commit_id: Oid) -> Result<TempRepo, Box<dyn Error>> {
        let max_attempts = 20;
        let tmp_dir = std::env::temp_dir();

        for suffix in 0..max_attempts {
            let tmp_repo_path = tmp_dir.join(format!("capn_tmp_{}_{}.git", commit_id, suffix));
            match std::fs::create_dir(&tmp_repo_path) {
                Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(Box::new(e)),
                Ok(_) => {
                    trace!(
                        "Created temp repo for verification: {}",
                        tmp_repo_path.display()
                    );
                    return Ok(TempRepo {
                        repo: Repository::init_bare(&tmp_repo_path)?,
                    });
                }
            };
        }

        Err(Box::new(CapnError::new(String::from(
            "Max attempts exceeded looking for a new temp repo location",
        ))))
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        trace!("Cleaning up temp repo: {}", self.repo.path().display());
        let drop_result = std::fs::remove_dir_all(&self.repo.path());
        if let Err(e) = drop_result {
            warn!("Failed to clean up temp repo: {}", e);
        }
    }
}
