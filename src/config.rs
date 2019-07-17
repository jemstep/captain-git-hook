use serde::{Deserialize, Serialize};
use toml;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Config {
    pub prepend_branch_name: Option<Unit>,
    pub verify_git_commits: Option<VerifyGitCommitsConfig>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct VerifyGitCommitsConfig {
    pub author_domain: String,
    pub committer_domain: String,
    pub keyserver: String,
    pub team_fingerprints_file: String,
    #[serde(default = "default_true")]
    pub recv_keys_par: bool,
    #[serde(default = "default_false")]
    pub skip_recv_keys: bool,
    #[serde(default = "default_true")]
    pub verify_email_addresses: bool,
    #[serde(default = "default_true")]
    pub verify_commit_signatures: bool,

    #[serde(default = "default_false")]
    pub verify_different_authors: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Unit {}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

impl Config {
    pub fn from_toml_string(input: &str) -> Result<Config, toml::de::Error> {
        toml::from_str(input)
    }
}
