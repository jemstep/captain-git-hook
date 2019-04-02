use std::path::PathBuf;
use structopt::StructOpt;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

mod git;
mod policies;
mod config;

use crate::policies::*;
use crate::config::Config;

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
    let config = git::read_config()?;

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
    use git2::Repository;
    use std::os::unix::fs::PermissionsExt;

    let git_repo = Repository::discover("./")?;
    let dotgit_dir = git_repo.path();
    let hook_dir = dotgit_dir.join("hooks");

    let mut prepare_commit_msg = File::create(hook_dir.join("prepare-commit-msg"))?;
    prepare_commit_msg.set_permissions(PermissionsExt::from_mode(0o750))?;

    writeln!(prepare_commit_msg, "#!/bin/sh")?;
    writeln!(prepare_commit_msg, "capn prepare-commit-msg \"$@\"")?;

    Ok(())
}

fn debug(config: Config) -> Result<(), Box<Error>> {
    println!("Captain Git Hook called with the following configuration:");
    println!("{:#?}", config);

    Ok(())
}
