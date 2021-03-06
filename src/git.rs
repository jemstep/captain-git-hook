use crate::error::CapnError;
use crate::keyring::Keyring;
use git2;
use git2::{ErrorClass, ErrorCode, ObjectType, Oid, Repository};
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process::*;
use std::str;
use uuid::Uuid;

use crate::config::*;
use log::*;

#[derive(Debug, Clone)]
pub struct Commit {
    pub id: Oid,
    pub author_email: Option<String>,
    pub committer_email: Option<String>,
    pub is_identical_tree_to_any_parent: bool,
    pub is_merge_commit: bool,
    pub tags: Vec<Tag>,
    pub parents: Vec<Oid>,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub id: Oid,
    pub name: String,
    pub tagger_email: Option<String>,
}

pub trait Git: Sized {
    fn read_file(&self, path: &str) -> Result<String, Box<dyn Error>>;
    fn write_git_file(
        &self,
        path: &str,
        file_mode: u32,
        contents: &str,
    ) -> Result<(), Box<dyn Error>>;
    fn current_branch(&self) -> Result<String, Box<dyn Error>>;
    fn is_tag(&self, ref_name: &str) -> Result<bool, Box<dyn Error>>;
    fn find_commit(
        &self,
        commit_id: Oid,
        override_tag_pattern: &Option<String>,
    ) -> Result<Commit, Box<dyn Error>>;
    fn find_new_commits(
        &self,
        exclusions: &[Oid],
        inclusions: &[Oid],
        override_tag_pattern: &Option<String>,
    ) -> Result<Vec<Commit>, Box<dyn Error>>;

    fn is_merge_commit(&self, commit_id: Oid) -> bool;
    fn is_trivial_merge_commit(&self, commit: &Commit) -> Result<bool, Box<dyn Error>>;
    fn is_mainline(&self, ref_name: &str) -> Result<bool, Box<dyn Error>>;
    fn path(&self) -> &Path;
    fn verify_commit_signature(
        path: &Path,
        commit: &Commit,
        keyring: &Keyring,
    ) -> Result<bool, Box<dyn Error>>;
    fn verify_tag_signature(
        path: &Path,
        tag: &Tag,
        keyring: &Keyring,
    ) -> Result<bool, Box<dyn Error>>;
    fn read_config(&self) -> Result<Config, Box<dyn Error>> {
        let config_str = self.read_file(".capn")?;
        let config = Config::from_toml_string(&config_str)?;
        Ok(config)
    }
    fn is_descendent_of(&self, commit: Oid, ancestor: Oid) -> Result<bool, Box<dyn Error>>;
}

pub struct LiveGit {
    repo: Repository,
    config: GitConfig,
    tag_cache: RefCell<HashMap<Option<String>, HashMap<Oid, Vec<Tag>>>>,
    temp_repo_clone: RefCell<Option<TempRepo>>,
}

impl Git for LiveGit {
    fn path(&self) -> &Path {
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

    fn find_commit(
        &self,
        commit_id: Oid,
        override_tag_pattern: &Option<String>,
    ) -> Result<Commit, Box<dyn Error>> {
        let commit = self.repo.find_commit(commit_id)?;
        let committer = commit.committer();
        let committer_email = committer.email().map(|s| s.to_string());
        let author = commit.author();
        let author_email = author.email().map(|s| s.to_string());

        let tags = self.get_tags(commit_id, override_tag_pattern);

        Ok(Commit {
            id: commit.id(),
            author_email: author_email,
            committer_email: committer_email,
            is_merge_commit: commit.parent_count() > 1,
            is_identical_tree_to_any_parent: Self::is_identical_tree_to_any_parent(&commit),
            tags: tags,
            parents: commit.parent_ids().collect(),
        })
    }

    fn find_new_commits(
        &self,
        exclusions: &[Oid],
        inclusions: &[Oid],
        override_tag_pattern: &Option<String>,
    ) -> Result<Vec<Commit>, Box<dyn Error>> {
        let mut revwalk = self.repo.revwalk()?;
        for &inclusion in inclusions.iter().filter(|id| !id.is_zero()) {
            revwalk.push(inclusion)?;
        }
        for &exclusion in exclusions.iter().filter(|id| !id.is_zero()) {
            revwalk.hide(exclusion)?;
        }
        for mainline in &self.config.mainlines {
            if mainline == "HEAD" {
                revwalk.hide_head()?;
            } else if mainline.contains(|c| c == '?' || c == '*' || c == '[') {
                revwalk.hide_glob(&format!("refs/heads/{}", mainline))?;
            } else {
                match revwalk.hide_ref(&format!("refs/heads/{}", mainline)) {
                    Ok(()) => {}
                    Err(e) if e.class() == ErrorClass::Reference => {
                        warn!("Failed to exclude mainline branch {}. Error: {}.\nThis could indicate that the branch doesn't exist.", mainline, e);
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }

        let commits = revwalk
            .into_iter()
            .map(|id| {
                id.map_err(|e| e.into())
                    .and_then(|id| self.find_commit(id, override_tag_pattern))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(commits)
    }

    fn verify_tag_signature(
        path: &Path,
        tag: &Tag,
        keyring: &Keyring,
    ) -> Result<bool, Box<dyn Error>> {
        let tag_id = &tag.id;

        let tagger_email = match &tag.tagger_email {
            Some(email) => email,
            None => {
                debug!(
                    "Tag {} does not have a valid tagger: no email address",
                    tag_id
                );
                return Ok(false);
            }
        };
        let expected_fingerprint = match keyring.fingerprint_id_from_email(tagger_email) {
            Some(f) => f,
            None => {
                debug!(
                    "Did not find GPG key for tag {}, tagger {}",
                    tag_id, tagger_email
                );
                return Ok(false);
            }
        };

        let result = Command::new("git")
            .current_dir(path)
            .arg("verify-tag")
            .arg("--raw")
            .arg(tag_id.to_string())
            .output()?;
        debug!(
            "Result from calling git verify-tag on {}: {:?}",
            tag_id, result
        );

        let encoded = String::from_utf8(result.stderr)?;

        let valid = encoded
            .split('\n')
            .any(|s| s.contains(&format!("VALIDSIG {}", expected_fingerprint)));

        if valid {
            debug!("Tag {} was signed with a valid signature", tag_id);
            Ok(true)
        } else {
            debug!("Tag {} was not signed with a valid signature", tag_id);
            Ok(false)
        }
    }

    fn verify_commit_signature(
        path: &Path,
        commit: &Commit,
        keyring: &Keyring,
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
        let expected_fingerprint = match keyring.fingerprint_id_from_email(committer_email) {
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

    fn is_merge_commit(&self, commit_id: Oid) -> bool {
        self.repo
            .find_commit(commit_id)
            .map(|c| c.parent_count() > 1)
            .unwrap_or(false)
    }

    fn is_trivial_merge_commit(
        &self,
        verification_commit: &Commit,
    ) -> Result<bool, Box<dyn Error>> {
        use git2::MergeOptions;

        self.with_temp_repo_clone(|temp_repo| {
            let commit = temp_repo.repo.find_commit(verification_commit.id)?;
            let parents = commit.parents().collect::<Vec<_>>();
            match &parents[..] {
                [a, b] => {
                    let expected_tree_id = commit.tree_id();
                    let reproduced_tree_id = temp_repo
                        .repo
                        .merge_commits(&a, &b, Some(MergeOptions::new().fail_on_conflict(true)))
                        .and_then(|mut index| index.write_tree_to(&temp_repo.repo));
                    trace!("Checking for a trivial merge commit, expecting tree_id of {}, result of reproducing tree is {:?}", expected_tree_id, reproduced_tree_id);
                    let matches = reproduced_tree_id
                        .as_ref()
                        .map(|id| *id == expected_tree_id)
                        .unwrap_or(false);

                    Ok(matches)
                }
                _ => Ok(false),
            }
        })
    }

    fn is_mainline(&self, ref_name: &str) -> Result<bool, Box<dyn Error>> {
        fn is_head(git: &LiveGit, ref_name: &str) -> Result<bool, Box<dyn Error>> {
            let head = git.repo.head()?;
            Ok(Some(ref_name) == head.name())
        }
        fn matches_glob(git: &LiveGit, ref_name: &str, glob: &str) -> Result<bool, Box<dyn Error>> {
            git.repo
                .references_glob(&format!("refs/heads/{}", glob))?
                .names()
                .map(|name| name.map(|n| n == ref_name))
                .fold(Ok(false), |acc, next| {
                    acc.and_then(|a| next.map(|b| a || b).map_err(|e| e.into()))
                })
        }

        self.config
            .mainlines
            .iter()
            .map(|mainline_glob| {
                if mainline_glob == "HEAD" {
                    is_head(self, ref_name)
                } else {
                    matches_glob(self, ref_name, mainline_glob)
                }
            })
            .fold(Ok(false), |acc, next| {
                acc.and_then(|a| next.map(|b| a || b))
            })
    }

    fn is_tag(&self, ref_name: &str) -> Result<bool, Box<dyn Error>> {
        match self.repo.find_reference(ref_name) {
            Ok(reference) => Ok(reference.is_tag()),
            Err(err) if err.code() == ErrorCode::NotFound => Ok(ref_name.starts_with("refs/tags")),
            Err(e) => Err(e.into()),
        }
    }

    fn is_descendent_of(&self, commit: Oid, ancestor: Oid) -> Result<bool, Box<dyn Error>> {
        self.repo
            .graph_descendant_of(commit, ancestor)
            .map_err(|e| e.into())
    }
}

impl LiveGit {
    pub fn default(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let repo = Repository::discover(path)?;
        Ok(LiveGit {
            repo,
            config: GitConfig::default(),
            tag_cache: RefCell::new(HashMap::new()),
            temp_repo_clone: RefCell::new(None),
        })
    }

    pub fn new(path: impl AsRef<Path>, config: GitConfig) -> Result<Self, Box<dyn Error>> {
        let repo = Repository::discover(path)?;
        Ok(LiveGit {
            repo,
            config,
            tag_cache: RefCell::new(HashMap::new()),
            temp_repo_clone: RefCell::new(None),
        })
    }

    fn is_identical_tree_to_any_parent(commit: &git2::Commit<'_>) -> bool {
        let tree_id = commit.tree_id();
        commit.parents().any(|p| p.tree_id() == tree_id)
    }

    fn get_tags(&self, commit_id: Oid, pattern: &Option<String>) -> Vec<Tag> {
        let mut tag_cache = self.tag_cache.borrow_mut();

        tag_cache
            .entry(pattern.clone())
            .or_insert_with(|| {
                self.repo
                    .tag_names(pattern.as_deref())
                    .ok()
                    .iter()
                    .flat_map(|tag_names| tag_names.iter().flatten())
                    .filter_map(|tag_name| {
                        self.repo
                            .revparse_single(tag_name)
                            .and_then(|git_obj| git_obj.peel_to_tag())
                            .ok()
                    })
                    .filter(|tag| tag.target_type() == Some(ObjectType::Commit))
                    .fold(HashMap::new(), |mut map, tag| {
                        map.entry(tag.target_id())
                            .or_insert_with(|| Vec::new())
                            .push(Tag {
                                id: tag.id(),
                                name: tag.name().map(|s| s.to_string()).unwrap_or(String::new()),
                                tagger_email: tag
                                    .tagger()
                                    .and_then(|signature| signature.email().map(|s| s.to_string())),
                            });
                        map
                    })
            })
            .get(&commit_id)
            .cloned()
            .unwrap_or(Vec::new())
    }

    fn with_temp_repo_clone<T>(
        &self,
        f: impl Fn(&TempRepo) -> Result<T, Box<dyn Error>>,
    ) -> Result<T, Box<dyn Error>> {
        let mut cache = self.temp_repo_clone.borrow_mut();
        if let Some(temp_repo) = cache.as_ref() {
            f(temp_repo)
        } else {
            let temp_repo = TempRepo::new(&self.repo)?;
            let result = f(&temp_repo);
            *cache = Some(temp_repo);
            result
        }
    }
}

struct TempRepo {
    repo: Repository,
}

impl TempRepo {
    fn new(src_repo: &Repository) -> Result<TempRepo, Box<dyn Error>> {
        let max_attempts = 20;
        let tmp_dir = std::env::temp_dir();

        for attempt in 1..=max_attempts {
            let tmp_repo_path = tmp_dir.join(format!("capn_tmp_{}.git", Uuid::new_v4()));
            match std::fs::create_dir(&tmp_repo_path) {
                Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(Box::new(e)),
                Ok(_) => {
                    trace!(
                        "Created temp repo for verification after {} attempts: {}",
                        attempt,
                        tmp_repo_path.display()
                    );
                    let src_path = src_repo.path().to_str().ok_or(Box::new(CapnError::new(
                        "Path to the repo being verified was not valid UTF-8",
                    )))?;

                    let result = Command::new("git")
                        .arg("clone")
                        .arg("--bare")
                        .arg("--shared")
                        .arg(src_path)
                        .arg(&tmp_repo_path)
                        .output()?;

                    return if result.status.success() {
                        Ok(TempRepo {
                            repo: Repository::open(tmp_repo_path)?,
                        })
                    } else {
                        debug!("Git clone Stderr: {:?}", String::from_utf8(result.stderr));
                        Err(Box::new(CapnError::new(format!(
                            "Call to git clone while creating temp repo failed with code {:?}",
                            result.status.code()
                        ))))
                    };
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

#[cfg(test)]
mod test {
    use super::*;
    use git2::{Oid, Reference};
    use quickcheck_macros::quickcheck;

    fn valid_mainlines(mainlines: &[String]) -> bool {
        mainlines
            .iter()
            .all(|mainline| !mainline.contains('\u{0}') && Reference::is_valid_name(&mainline))
    }

    #[test]
    fn is_mainline_with_default_config_only_identifies_head_branch() {
        let project_root = env!("CARGO_MANIFEST_DIR");
        let git = LiveGit::default(format!("{}/tests/test-repo.git", project_root)).unwrap();
        assert_eq!(git.is_mainline("refs/heads/master").unwrap(), true);
        assert_eq!(git.is_mainline("refs/heads/tagged-branch").unwrap(), false);
    }

    #[test]
    fn is_mainline_with_glob_config_does_not_identify_head_branch() {
        let project_root = env!("CARGO_MANIFEST_DIR");
        let git = LiveGit::new(
            format!("{}/tests/test-repo.git", project_root),
            GitConfig {
                mainlines: vec!["tagged-*".into()],
            },
        )
        .unwrap();
        assert_eq!(git.is_mainline("refs/heads/master").unwrap(), false);
        assert_eq!(git.is_mainline("refs/heads/tagged-branch").unwrap(), true);
    }

    #[test]
    fn is_mainline_with_literal_config_does_not_identify_head_branch() {
        let project_root = env!("CARGO_MANIFEST_DIR");
        let git = LiveGit::new(
            format!("{}/tests/test-repo.git", project_root),
            GitConfig {
                mainlines: vec!["tagged-branch".into()],
            },
        )
        .unwrap();
        assert_eq!(git.is_mainline("refs/heads/master").unwrap(), false);
        assert_eq!(git.is_mainline("refs/heads/tagged-branch").unwrap(), true);
    }

    #[quickcheck]
    fn is_mainline_fuzz(branch: String, mainlines: Vec<String>) {
        if valid_mainlines(&mainlines) {
            let project_root = env!("CARGO_MANIFEST_DIR");
            let git = LiveGit::new(
                format!("{}/tests/test-repo.git", project_root),
                GitConfig { mainlines },
            )
            .unwrap();
            git.is_mainline(&branch).unwrap();
        }
    }

    #[test]
    fn is_mainline_with_multiple_glob_config_identifies_all_matches() {
        let project_root = env!("CARGO_MANIFEST_DIR");
        let git = LiveGit::new(
            format!("{}/tests/test-repo.git", project_root),
            GitConfig {
                mainlines: vec!["HEAD".into(), "tagged-*".into()],
            },
        )
        .unwrap();
        assert_eq!(git.is_mainline("refs/heads/master").unwrap(), true);
        assert_eq!(git.is_mainline("refs/heads/tagged-branch").unwrap(), true);
    }

    #[quickcheck]
    fn new_commits_fuzz(mainlines: Vec<String>) {
        if valid_mainlines(&mainlines) {
            let project_root = env!("CARGO_MANIFEST_DIR");
            let git = LiveGit::new(
                format!("{}/tests/test-repo.git", project_root),
                GitConfig { mainlines },
            )
            .unwrap();
            git.find_new_commits(
                &[Oid::from_str("eb5e0185546b0bb1a13feec6b9ee8b39985fea42").unwrap()],
                &[Oid::from_str("6004dfdb071c71e5e76ad55b924b576487e1c485").unwrap()],
                &None,
            )
            .unwrap();
        }
    }

    mod is_tag {
        use super::super::*;

        #[test]
        fn identifies_existing_branch_as_not_a_tag() {
            let project_root = env!("CARGO_MANIFEST_DIR");
            let git = LiveGit::default(format!("{}/tests/test-repo.git", project_root)).unwrap();
            assert_eq!(git.is_tag("refs/heads/master").ok(), Some(false));
        }
        #[test]
        fn identifies_new_branch_as_not_a_tag() {
            let project_root = env!("CARGO_MANIFEST_DIR");
            let git = LiveGit::default(format!("{}/tests/test-repo.git", project_root)).unwrap();
            assert_eq!(
                git.is_tag("refs/heads/new-brach-that-does-not-exist").ok(),
                Some(false)
            );
        }
        #[test]
        fn identifies_existing_lightweight_tag_as_a_tag() {
            let project_root = env!("CARGO_MANIFEST_DIR");
            let git = LiveGit::default(format!("{}/tests/test-repo.git", project_root)).unwrap();
            assert_eq!(git.is_tag("refs/tags/lightweight-tag").ok(), Some(true));
        }
        #[test]
        fn identifies_existing_annotated_tag_as_a_tag() {
            let project_root = env!("CARGO_MANIFEST_DIR");
            let git = LiveGit::default(format!("{}/tests/test-repo.git", project_root)).unwrap();
            assert_eq!(
                git.is_tag("refs/tags/capn-override-test-user-1").ok(),
                Some(true)
            );
        }
        #[test]
        fn identifies_new_tag_as_a_tag() {
            let project_root = env!("CARGO_MANIFEST_DIR");
            let git = LiveGit::default(format!("{}/tests/test-repo.git", project_root)).unwrap();
            assert_eq!(
                git.is_tag("refs/tags/new-tag-that-does-not-exist").ok(),
                Some(true)
            );
        }
    }

    mod find_new_commits {
        use super::super::*;
        use git2::Oid;

        // These tests are running on these commits:
        //
        // *   6004dfd (same-author) Merge branch 'valid-branch' into same-author
        // |\
        // | * 26b9047 (valid-branch) Valid commit on a branch
        // |/
        // * eb5e018 (HEAD -> master) A second valid commit

        #[test]
        fn finds_all_commits_ahead_of_head_with_default_config() {
            let project_root = env!("CARGO_MANIFEST_DIR");
            let git = LiveGit::default(format!("{}/tests/test-repo.git", project_root)).unwrap();
            let commits = git
                .find_new_commits(
                    &[Oid::from_str("eb5e0185546b0bb1a13feec6b9ee8b39985fea42").unwrap()],
                    &[Oid::from_str("6004dfdb071c71e5e76ad55b924b576487e1c485").unwrap()],
                    &None,
                )
                .unwrap();
            assert_eq!(commits.len(), 2)
        }

        #[test]
        fn finds_only_commits_in_none_of_the_mainlines_with_glob_configuration() {
            let project_root = env!("CARGO_MANIFEST_DIR");
            let git = LiveGit::new(
                format!("{}/tests/test-repo.git", project_root),
                GitConfig {
                    mainlines: vec!["HEAD".into(), "valid-*".into()],
                },
            )
            .unwrap();
            let commits = git
                .find_new_commits(
                    &[Oid::from_str("eb5e0185546b0bb1a13feec6b9ee8b39985fea42").unwrap()],
                    &[Oid::from_str("6004dfdb071c71e5e76ad55b924b576487e1c485").unwrap()],
                    &None,
                )
                .unwrap();
            assert_eq!(commits.len(), 1)
        }

        #[test]
        fn finds_only_commits_in_none_of_the_mainlines_with_literal_branch_name_configuration() {
            let project_root = env!("CARGO_MANIFEST_DIR");
            let git = LiveGit::new(
                format!("{}/tests/test-repo.git", project_root),
                GitConfig {
                    mainlines: vec!["HEAD".into(), "valid-branch".into()],
                },
            )
            .unwrap();
            let commits = git
                .find_new_commits(
                    &[Oid::from_str("eb5e0185546b0bb1a13feec6b9ee8b39985fea42").unwrap()],
                    &[Oid::from_str("6004dfdb071c71e5e76ad55b924b576487e1c485").unwrap()],
                    &None,
                )
                .unwrap();
            assert_eq!(commits.len(), 1)
        }

        #[test]
        fn finds_only_commits_in_none_of_the_mainlines_with_missing_branch_in_configuration() {
            let project_root = env!("CARGO_MANIFEST_DIR");
            let git = LiveGit::new(
                format!("{}/tests/test-repo.git", project_root),
                GitConfig {
                    mainlines: vec!["HEAD".into(), "this-branch-does-not-exist-asdfg".into()],
                },
            )
            .unwrap();
            let commits = git
                .find_new_commits(
                    &[Oid::from_str("eb5e0185546b0bb1a13feec6b9ee8b39985fea42").unwrap()],
                    &[Oid::from_str("6004dfdb071c71e5e76ad55b924b576487e1c485").unwrap()],
                    &None,
                )
                .unwrap();
            assert_eq!(commits.len(), 2)
        }
    }
}
