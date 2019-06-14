use structopt::StructOpt;
use std::error::Error;
use std::process::exit;

use capn::git::{LiveGit, Git};
use capn::gpg::LiveGpg;
use capn::fs::LiveFs;
use capn::pretty::*;
use capn::config::Config;
use capn::*;
use capn::policies::PolicyResult;

use stderrlog;
use log::*;

use std::io::stdin;
use std::io::prelude::*;

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

    /// Git hook called before pushing to a remote repo
    #[structopt(name = "pre-push")]
    PrePush(PrePush),

    /// Git hook called on the server before updating any references
    #[structopt(name = "pre-receive")]
    PreReceive,

    /// Installs the required Git Hooks in the current repo
    #[structopt(name = "install-hooks")]
    InstallHooks,
}

// This function intentionally doesn't return 'error', it's meant to
// nicely log any errors that happened further down and, if there are
// errors, exit with a non-zero code.
fn main() {
    let opt = Opt::from_args();

    stderrlog::new()
        .module(module_path!())
        .quiet(opt.quiet)
        .verbosity(opt.verbose + 2) // Default is info (2)
        .init()
        .expect("ERROR: Logger was initialized twice");
    
    info!("{}", block("Ahoy, maties! Welcome to Capn Githook!"));

    let git = match LiveGit::new() {
        Ok(g) => g,
        Err(e) => {
            error!("Capn Githook must be called in a Git repo. Error: {}", e);
            exit(1);
        }
    };

    let config = match git.read_config() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to read the .capn config file. Please check that you are in a Git repo that has a .capn config file in the root of the repo. Error: {}", e);
            exit(1);
        }
    };

    debug!("Configuration: {:#?}\n", config);

    match execute_command(opt.command, config) {
        Ok(()) => {
            info!("{}", block("Aye, me hearties! Welcome aboard!"));
        },
        Err(e) => {
            error!("{}", e);
            exit(1);
        }
    }
}

fn execute_command(command: Command, config: Config) -> Result<(), Box<dyn Error>> {
    match command {
        Command::PrepareCommitMsg(x) => {
            info!("Calling prepare-commit-msg");
            prepare_commit_msg::<LiveFs, LiveGit>(x, config).map(|x| {
                match x {
                    PolicyResult::Ok => {
                    },
                    _e => {
                    }
                };
                ()
            })
        },
        Command::PrePush(x) => {
            for raw_line in stdin().lock().lines() {
                let line = raw_line?;
                let mut fields = line.split(' ');
                match (fields.next(), fields.next(), fields.next(), fields.next()) {
                    (Some(local_ref), Some(local_sha), Some(remote_ref), Some(remote_sha)) => {
                        info!("Calling prepush with: {} {} {} {}", local_ref, local_sha, remote_ref, remote_sha);
                        pre_push::<LiveGit, LiveGpg>(&x, &config, local_ref, local_sha, remote_ref, remote_sha)?;
                    },
                    _ => {
                        warn!("Expected parameters not received on stdin. Line received was: {}", line);
                    }
                };
            }
            Ok(())
        },
         Command::PreReceive => {
            for raw_line in stdin().lock().lines() {
                let line = raw_line?;
                let mut fields = line.split(' ');
                match (fields.next(), fields.next(), fields.next()) {
                    (Some(old_value), Some(new_value), Some(ref_name)) => {
                        info!("Calling prereceive with: {} {} {}", old_value, new_value, ref_name);
                        pre_receive::<LiveGit, LiveGpg>(&config, old_value, new_value, ref_name)?;
                    },
                    _ => {
                        warn!("Expected parameters not received on stdin. Line received was: {}", line);
                    }
                };
            }
            Ok(())
        },
        Command::InstallHooks => install_hooks::<LiveGit>(),
    }
}
