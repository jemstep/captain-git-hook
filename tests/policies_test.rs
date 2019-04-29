use capn::config::VerifyGitCommitsConfig;
use capn::policies;
use capn::git::LiveGit;
use capn::gpg::LiveGpg;

#[test]
#[ignore("This functionality has not been implemented yet")]
fn verify_git_commits() {
    let config = VerifyGitCommitsConfig {
        author_domain = "jemstep.com", 
        committer_domain = "jemstep.com",
        
    }

      pub author_domain: String,
    pub committer_domain: String,
    pub keyserver: String,
    pub team_fingerprints_file: String,
    pub recv_keys_par: bool
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(VerifyGitCommitsConfig{}, "old_value","new_value","ref_name","gpg/TEAM_FINGERPRINTS");
    assert!(result.is_ok());
}
