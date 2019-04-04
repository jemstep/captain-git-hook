use std::path::PathBuf;
use structopt::StructOpt;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::process::exit;
use git2::Repository;

mod git;
mod gpg;
mod policies;
mod config;

use crate::policies::*;
use crate::config::Config;
use crate::git::{LiveGit, Git};

#[derive(Debug, StructOpt)]
#[structopt(name = "Captain Git Hook", about = "A collection of tools for more opinionated Git usage")]
enum Opt {
    /// Git hook called before opening an editor with a commit message
    #[structopt(name = "prepare-commit-msg")]
    PrepareCommitMsg(PrepareCommitMsg),

    /// Git hook called on the server before updating any references
    #[structopt(name = "pre-receive")]
    PreReceive,

    /// Installs the required Git Hooks in the current repo
    #[structopt(name = "install-hooks")]
    InstallHooks,

    /// Logs the current configuration and exists
    #[structopt(name = "debug")]
    Debug
}

#[derive(Debug, StructOpt)]
struct PrepareCommitMsg {
    /// Commit file currently prepared by git
    #[structopt(parse(from_os_str))]
    commit_file: PathBuf,
    /// The source of the commit
    #[structopt()]
    commit_source: Option<String>,
    /// The SHA-1 of an existing commit (if -c, -C or --amend are being used with a commit)
    #[structopt()]
    existing_commit: Option<String>,
}

fn main() -> Result<(), Box<Error>> {
    let opt = Opt::from_args();
    let config = match LiveGit::new()?.read_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: Failed to read the .capn config file.");
            eprintln!("{}", e);
            exit(1);
        }
    };

    match opt {
        Opt::PrepareCommitMsg(x) => prepare_commit_msg(x, config),
        Opt::PreReceive => pre_receive(config),
        Opt::InstallHooks => install_hooks(),
        Opt::Debug => debug(config)
    }
}

fn prepare_commit_msg(opt: PrepareCommitMsg, config: Config) -> Result<(), Box<Error>> {
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

fn pre_receive(_config: Config) -> Result<(), Box<Error>> {
    Ok(())
}

fn install_hooks() -> Result<(), Box<Error>> {
    let repo = LiveGit::new()?;
    repo.write_git_file("hooks/prepare-commit-msg", r#"#!/bin/sh
capn prepare-commit-msg "$@""#)?;
    
    Ok(())
}

fn debug(config: Config) -> Result<(), Box<Error>> {
    println!("Captain Git Hook called with the following configuration:");
    println!("{:#?}", config);

    Ok(())
}
