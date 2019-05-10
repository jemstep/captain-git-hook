use capn::config::VerifyGitCommitsConfig;
use capn::policies;
use capn::git::LiveGit;
use capn::gpg::LiveGpg;

#[test]
#[ignore("This functionality has not been implemented yet")]
fn verify_git_commits() {
    let config = VerifyGitCommitsConfig {
        author_domain : "jemstep.com".to_string(), 
        committer_domain : "jemstep.com".to_string(),
        keyserver : "KEYSERVER".to_string(),
        team_fingerprints_file: "gpg/TEAM_FINGERPRINTS".to_string(),
        recv_keys_par : true,
        verify_email_addresses: true,
        verify_commit_signatures: true,
        verify_different_authors: true
    };

    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&config, "old_value","new_value","ref_name");
    assert!(result.is_ok());
}
