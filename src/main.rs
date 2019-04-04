use structopt::StructOpt;
use std::error::Error;
use std::process::exit;

use capn::git::{LiveGit, Git};
use capn::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "Captain Git Hook", about = "A collection of tools for more opinionated Git usage")]
pub enum Opt {
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
