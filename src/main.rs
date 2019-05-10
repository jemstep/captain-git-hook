use structopt::StructOpt;
use std::error::Error;
use std::process::exit;

use capn::git::{LiveGit, Git};
use capn::gpg::LiveGpg;
use capn::fs::LiveFs;
use capn::*;

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

fn main() -> Result<(), Box<dyn Error>> {

    let opt = Opt::from_args();
    stderrlog::new()
        .module(module_path!())
        .quiet(opt.quiet)
        .verbosity(opt.verbose + 2) // Default is info
        .init()?;
    
    info!("Aargh, Matey! Welcome to Capn Githook!");

    let config = match LiveGit::new()?.read_config() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to read the .capn config file: {}", e);
            exit(1);
        }
    };

    debug!("Configuration: {:#?}", config);

    match opt.command {
        Command::PrepareCommitMsg(x) => prepare_commit_msg::<LiveFs, LiveGit>(x, config),
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
