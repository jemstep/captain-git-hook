use std::error::Error;
use std::process::exit;
use structopt::StructOpt;

use capn::config::Config;
use capn::fs::LiveFs;
use capn::git::{Git, LiveGit};
use capn::gpg::{Gpg, LiveGpg};
use capn::logger;
use capn::logger::{Logger, LoggingOpt};
use capn::policies::PolicyResult;
use capn::*;

use log::*;
use std::io::prelude::*;
use std::io::stdin;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Captain Git Hook",
    about = "A collection of tools for more opinionated Git usage"
)]
pub struct Opt {
    #[structopt(flatten)]
    logging: LoggingOpt,
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
    let quiet = opt.logging.quiet;
    Logger::init(opt.logging);

    logger::print_header(
        format!(
            "Ahoy, maties! Welcome to Capn Githook {}!",
            env!("CARGO_PKG_VERSION")
        ),
        quiet,
    );

    let config = match load_config() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to initialize Capn Githook. Error: {}.\nPlease check that you are in a Git repo that has a .capn config file in the root of the repo.", e);
            exit(1);
        }
    };

    debug!("Configuration: {:#?}\n", config);

    match execute_command(opt.command, config) {
        Ok(PolicyResult::Ok) => {
            info!("Checks passed - commits accepted");
            logger::print_header("Aye, me hearties! Welcome aboard!", quiet);
        }
        Ok(e) => {
            error!("Checks failed - commits rejected - reason: {}", e);
            logger::print_header(format!("Your commits are scallywags!\n{}", e), quiet);
            exit(1);
        }
        Err(e) => {
            error!("System error - commits rejected - reason: {}", e);
            logger::print_header(format!("Something went wrong!\n{}", e), quiet);
            exit(1);
        }
    }
}

fn load_config() -> Result<Config, Box<dyn Error>> {
    // This is a necessary bootstrapping step, because we need a Git
    // object to load the config, which is used to initialize the Git
    // object used for the rest of the run.
    let default_git = LiveGit::default("./")?;
    default_git.read_config()
}

fn execute_command(command: Command, config: Config) -> Result<PolicyResult, Box<dyn Error>> {
    let git = LiveGit::new("./", config.git.clone())?;
    match command {
        Command::PrepareCommitMsg(args) => {
            info!("Calling prepare-commit-msg");
            prepare_commit_msg::<LiveFs, LiveGit>(&git, args, config)
        }
        Command::PrePush(args) => {
            info!("Calling pre-push");
            stdin().lock().lines()
                .map(|raw_line| raw_line.map(|line| {
                    let mut fields = line.split(' ');
                    match (fields.next(), fields.next(), fields.next(), fields.next()) {
                        (Some(local_ref), Some(local_sha), Some(remote_ref), Some(remote_sha)) => {
                            info!("Running pre-push for: {} {} {} {}", local_ref, local_sha, remote_ref, remote_sha);
                            pre_push::<LiveGit, _>(&git, build_gpg_client(&config), &args, &config, local_ref, local_sha, remote_ref, remote_sha)
                        },
                        _ => {
                            warn!("Expected parameters not received on stdin. Line received was: {}", line);
                            Ok(PolicyResult::Ok)
                        }
                    }
                }))
                .flatten()
                .collect()
        }
        Command::PreReceive => {
            info!("Calling pre-receive");
            stdin().lock().lines()
                .map(|raw_line| raw_line.map(|line| {
                    let mut fields = line.split(' ');
                    match (fields.next(), fields.next(), fields.next()) {
                        (Some(old_value), Some(new_value), Some(ref_name)) => {
                            info!("Running pre-receive for: {} {} {}", old_value, new_value, ref_name);
                            pre_receive::<LiveGit, _>(&git, build_gpg_client(&config), &config, old_value, new_value, ref_name)
                        },
                        _ => {
                            warn!("Expected parameters not received on stdin. Line received was: {}", line);
                            Ok(PolicyResult::Ok)
                        }
                    }
                }))
                .flatten()
                .collect()
        }
        Command::InstallHooks => install_hooks(git).map(|_| PolicyResult::Ok),
    }
}

fn build_gpg_client(config: &Config) -> impl Gpg {
    LiveGpg {
        parallel_fetch: config
            .verify_git_commits
            .as_ref()
            .map(|c| c.recv_keys_par)
            .unwrap_or(true),
        keyserver: config
            .verify_git_commits
            .as_ref()
            .map(|c| c.keyserver.clone())
            .unwrap_or("".to_string()),
    }
}
