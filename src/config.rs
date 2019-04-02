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
    pub committer_domain: String
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Unit {}

impl Config {
    pub fn from_toml_string(input: &str) -> Result<Config, toml::de::Error> {
        toml::from_str(input)
    }
}
