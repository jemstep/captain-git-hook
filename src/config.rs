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
    #[serde(default = "default_true")]
    pub verify_email_addresses: bool,
    #[serde(default = "default_true")]
    pub verify_commit_signatures: bool,

    #[serde(default = "default_false")]
    pub verify_different_authors: bool,

    #[serde(default)]
    pub override_tag_pattern: Option<String>,
    #[serde(default = "default_two")]
    pub override_tags_required: u8,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Unit {}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_two() -> u8 {
    2
}

impl Config {
    pub fn from_toml_string(input: &str) -> Result<Config, toml::de::Error> {
        toml::from_str(input)
    }
}
