use std::path::PathBuf;
use structopt::StructOpt;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

mod git;
mod policies;
use crate::policies::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "Captain Git Hook", about = "A collection of tools for more opinionated Git usage")]
enum Opt {
    /// Git hook called before opening an editor with a commit message
    #[structopt(name = "prepare-commit-msg")]
    PrepareCommitMsg(PrepareCommitMsg),

    #[structopt(name = "install-hooks")]
    InstallHooks,
}

#[derive(Debug, StructOpt)]
struct PrepareCommitMsg {
    /// Commit file currently prepared by git
    #[structopt(parse(from_os_str))]
    commit_file: PathBuf,
    /// The source of the commit
    #[structopt()]
    commit_source: Option<String>,
}

fn main() -> Result<(), Box<Error>> {
    let opt = Opt::from_args();

    match opt {
        Opt::PrepareCommitMsg(x) => prepare_commit_msg(x),
        Opt::InstallHooks => install_hooks(),
    }
}

fn prepare_commit_msg(opt: PrepareCommitMsg) -> Result<(), Box<Error>> {
    if opt.commit_source.is_none() {
        prepend_branch_name(opt.commit_file)
    } else {
        // do nothing silently. This comes up on merge commits,
        // ammendment commits, if a message was specified on the
        // cli.
        Ok(())
    }
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
