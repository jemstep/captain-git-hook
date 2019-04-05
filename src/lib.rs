use std::error::Error;
use structopt::StructOpt;
use std::path::PathBuf;

use crate::policies::*;
use crate::config::Config;
use crate::git::{LiveGit, Git};

pub mod git;
pub mod gpg;
pub mod policies;
pub mod config;
mod error;

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


pub fn prepare_commit_msg(opt: PrepareCommitMsg, config: Config) -> Result<(), Box<Error>> {
    if opt.commit_source.is_none() {
        if let Some(_) = config.prepend_branch_name {
            prepend_branch_name(opt.commit_file)?;
        }

        Ok(())
    } else {
        // do nothing silently. This comes up on merge commits,
        // ammendment commits, if a message was specified on the
        // cli.
        Ok(())
    }
}

pub fn pre_receive(_config: Config) -> Result<(), Box<Error>> {
    Ok(())
}

pub fn install_hooks() -> Result<(), Box<Error>> {
    let repo = LiveGit::new()?;
    repo.write_git_file("hooks/prepare-commit-msg", r#"#!/bin/sh
capn prepare-commit-msg "$@"
"#)?;
    
    Ok(())
}

pub fn debug(config: Config) -> Result<(), Box<Error>> {
    println!("Captain Git Hook called with the following configuration:");
    println!("{:#?}", config);

    Ok(())
}
