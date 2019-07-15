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
use capn::logger::Logger;

use std::io::stdin;
use std::io::prelude::*;
use log::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "Captain Git Hook", about = "A collection of tools for more opinionated Git usage")]
pub struct Opt {
    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,
    /// Verbose mode (-v, -vv, -vvv, etc)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,
    /// URL for logging over TCP
    #[structopt(long = "log-url")]
    log_url: Option<String>,
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

    Logger::init(
        opt.quiet, opt.verbose,
        opt.log_url
    );
    
    info!("{}", block("Ahoy, maties! Welcome to Capn Githook!"));

    let git = match LiveGit::new() {
        Ok(g) => g,
        Err(e) => {
            error!("Failed to initialize Capn Githook. Error: {}\nPlease check that you are in a Git repo.", e);
            exit(1);
        }
    };

    let config = match git.read_config() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to read the .capn config file.  Error: {}.\nPlease check that you are in a Git repo that has a .capn config file in the root of the repo.", e);
            exit(1);
        }
    };

    debug!("Configuration: {:#?}\n", config);

    match execute_command(opt.command, config) {
        Ok(PolicyResult::Ok) => {
            info!("{}", block("Aye, me hearties! Welcome aboard!"));
        },
        Ok(e)  => {
            error!("{}", block(e));
            exit(1);
        },
        Err(e) => {
            error!("System error: {}", e);
            exit(1);
        }
    }
}

fn execute_command(command: Command, config: Config) -> Result<PolicyResult, Box<dyn Error>> {
    match command {
        Command::PrepareCommitMsg(args) => {
            info!("Calling prepare-commit-msg");
            prepare_commit_msg::<LiveFs, LiveGit>(args, config)
        },
        Command::PrePush(args) => {
            stdin().lock().lines()
                .map(|raw_line| raw_line.map(|line| {
                    let mut fields = line.split(' ');
                    match (fields.next(), fields.next(), fields.next(), fields.next()) {
                        (Some(local_ref), Some(local_sha), Some(remote_ref), Some(remote_sha)) => {
                            info!("Calling prepush with: {} {} {} {}", local_ref, local_sha, remote_ref, remote_sha);
                            pre_push::<LiveGit, LiveGpg>(&args, &config, local_ref, local_sha, remote_ref, remote_sha)
                        },
                        _ => {
                            warn!("Expected parameters not received on stdin. Line received was: {}", line);
                            Ok(PolicyResult::Ok)
                        }
                    }
                }))
                .flatten()
                .collect()
        },
        Command::PreReceive => {
            stdin().lock().lines()
                .map(|raw_line| raw_line.map(|line| {
                    let mut fields = line.split(' ');
                    match (fields.next(), fields.next(), fields.next()) {
                        (Some(old_value), Some(new_value), Some(ref_name)) => {
                            info!("Calling prereceive with: {} {} {}", old_value, new_value, ref_name);
                            pre_receive::<LiveGit, LiveGpg>(&config, old_value, new_value, ref_name)
                        },
                        _ => {
                            warn!("Expected parameters not received on stdin. Line received was: {}", line);
                            Ok(PolicyResult::Ok)
                        }
                    }
                }))
                .flatten()
                .collect()
        },
        Command::InstallHooks => {
            install_hooks::<LiveGit>()
                .map(|_| PolicyResult::Ok)
        },
    }
}
