#![deny(nonstandard_style)]
#![deny(warnings)]
#![deny(rust_2018_idioms)]
#![deny(unused)]
#![deny(future_incompatible)]

use std::error::Error;
use structopt::StructOpt;
use std::path::PathBuf;

use crate::policies::*;
use crate::config::Config;
use crate::git::Git;
use crate::fs::Fs;
use crate::gpg::Gpg;

pub mod git;
pub mod gpg;
pub mod policies;
pub mod config;
pub mod error;
pub mod fs;
pub mod fingerprints;
pub mod pretty;

#[derive(Debug, StructOpt)]
pub struct PrepareCommitMsg {
    /// Commit file currently prepared by git
    #[structopt(parse(from_os_str))]
    pub commit_file: PathBuf,
    /// The source of the commit
    #[structopt()]
    pub commit_source: Option<String>,
    /// The SHA-1 of an existing commit (if -c, -C or --amend are being used with a commit)
    #[structopt()]
    pub existing_commit: Option<String>,
}

#[derive(Debug, StructOpt)]
pub struct PrePush {
    /// The name of the destination remote
    #[structopt()]
    pub remote_name: String,
    /// The location of the destination remote
    #[structopt()]
    pub remote_location: String,
}

pub fn prepare_commit_msg<F: Fs, G: Git>(opt: PrepareCommitMsg, config: Config) -> Result<PolicyResult, Box<dyn Error>> {
    if opt.commit_source.is_none() {
        compose(vec![
            config.prepend_branch_name.map(|_| prepend_branch_name::<F, G>(opt.commit_file))
        ])
    } else {
        // do nothing silently. This comes up on merge commits,
        // ammendment commits, if a message was specified on the
        // cli.
        Ok(PolicyResult::Ok)
    }
}

pub fn pre_push<G: Git, P: Gpg>(_opt: &PrePush, config: &Config, _local_ref: &str, local_sha: &str, _remote_ref: &str, remote_sha: &str) -> Result<PolicyResult, Box<dyn Error>> {
    compose(vec![
        config.verify_git_commits.as_ref().map(|c| verify_git_commits::<G, P>(c, remote_sha, local_sha))
    ])
}

pub fn pre_receive<G: Git, P: Gpg>(config: &Config, old_value: &str, new_value: &str, _ref_name: &str) -> Result<PolicyResult, Box<dyn Error>> {
    compose(vec![
        config.verify_git_commits.as_ref().map(|c| verify_git_commits::<G, P>(c, old_value, new_value))
    ])
}

pub fn install_hooks<G: Git>() -> Result<(), Box<dyn Error>> {
    let repo = G::new()?;
    repo.write_git_file("hooks/prepare-commit-msg", 0o750, r#"#!/bin/sh
capn prepare-commit-msg "$@"
"#)?;
    repo.write_git_file("hooks/pre-push", 0o750, r#"#!/bin/sh
capn pre-push "$@"
"#)?;
    Ok(())
}


fn compose(results: Vec<Option<Result<PolicyResult, Box<dyn Error>>>>) -> Result<PolicyResult, Box<dyn Error>> {
    results.into_iter()
        .filter_map(|x| x)
        .fold(Ok(PolicyResult::Ok), |acc, next| acc.and_then(|a| next.map(|b| a.and(b))))
}
