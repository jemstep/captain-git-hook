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


pub fn prepare_commit_msg<F: Fs, G: Git>(opt: PrepareCommitMsg, config: Config) -> Result<(), Box<dyn Error>> {
    if opt.commit_source.is_none() {
        if let Some(_) = config.prepend_branch_name {
            prepend_branch_name::<F, G>(opt.commit_file)?;
        }

        Ok(())
    } else {
        // do nothing silently. This comes up on merge commits,
        // ammendment commits, if a message was specified on the
        // cli.
        Ok(())
    }
}

pub fn pre_push(_opt: &PrePush, _config: &Config, _local_ref: &str, _local_sha: &str, _remote_ref: &str, _remote_sha: &str) -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub fn pre_receive<G: Git, P: Gpg>(config: Config, new_value: &str) -> Result<(), Box<dyn Error>> {
    if let Some(c) = config.verify_git_commits {
        verify_git_commits::<G, P>(new_value, &c.team_fingerprints_file , &c.keyserver)?;
    }
    Ok(())
}

pub fn install_hooks<G: Git>() -> Result<(), Box<dyn Error>> {
    let repo = G::new()?;
    repo.write_git_file("hooks/prepare-commit-msg", 0o750, r#"#!/bin/sh
capn prepare-commit-msg "$@"
"#)?;
    
    Ok(())
}
