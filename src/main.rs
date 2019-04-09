use structopt::StructOpt;
use std::error::Error;
use std::process::exit;

use capn::git::{LiveGit, Git};
use capn::*;

use stderrlog;

#[derive(Debug, StructOpt)]
#[structopt(name = "Captain Git Hook", about = "A collection of tools for more opinionated Git usage")]
pub struct Opt {
    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,
    /// Verbose mode (-v, -vv, -vvv, etc)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,
    /// Command to be run
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
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
    stderrlog::new()
        .module(module_path!())
        .quiet(opt.quiet)
        .verbosity(opt.verbose)
        .init()?;

    let config = match LiveGit::new()?.read_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: Failed to read the .capn config file.");
            eprintln!("{}", e);
            exit(1);
        }
    };

    match opt.command {
        Command::PrepareCommitMsg(x) => prepare_commit_msg(x, config),
        Command::PreReceive => pre_receive(config, "new_value"),
        Command::InstallHooks => install_hooks(),
        Command::Debug => debug(config)
    }
}
