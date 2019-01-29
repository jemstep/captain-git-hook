use std::path::PathBuf;
use structopt::StructOpt;
use std::error::Error;

mod git;
mod policies;
use crate::policies::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "captain-git-hook", about = "An example of StructOpt usage.")]
enum Opt {
    /// Git hook called before opening an editor with a commit message
    #[structopt(name = "prepare-commit-msg")]
    PrepareCommitMsg(PrepareCommitMsg),
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
        Opt::PrepareCommitMsg(x) => prepare_commit_msg(x)
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

