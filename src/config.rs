use serde::{Serialize, Deserialize};
use toml;


#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Config {
    pub prepend_branch_name: Option<Unit>,
    pub example_complex_config: Option<ExampleComplexConfig>,
    pub verify_git_commits: Option<VerifyGitCommitsConfig>
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ExampleComplexConfig {
    pub command: String,
    pub repeats: u32
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct VerifyGitCommitsConfig {
    pub author_domain: String,
    pub committer_domain: String,
    pub keyserver: String,
    pub team_fingerprints_file: String,
    pub recv_keys_par: bool,
    pub verify_email_addresses: bool,
    pub verify_commit_signatures: bool,
    pub verify_different_authors: bool
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Unit {}

impl Config {
    pub fn from_toml_string(input: &str) -> Result<Config, toml::de::Error> {
        toml::from_str(input)
    }
}
