use capn::config::VerifyGitCommitsConfig;
use capn::policies;
use capn::git::LiveGit;
use capn::gpg::LiveGpg;

#[test]
fn verify_git_commits() {
    std::env::set_current_dir("./tests/test-repo.git").unwrap();
    
    let config = VerifyGitCommitsConfig {
        author_domain : "jemstep.com".to_string(), 
        committer_domain : "jemstep.com".to_string(),
        keyserver : "hkp://pgp.jemstep.com:80".to_string(),
        team_fingerprints_file: "TEAM_FINGERPRINTS".to_string(),
        recv_keys_par : true,
        verify_email_addresses: true,
        verify_commit_signatures: true,
        verify_different_authors: true
    };

    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&config, "0000000000000000000000000000000000000000","69841d34d7dbef6d70ea9f59419c9fed7749575f","master");
    assert!(result.is_ok(), "Error: {:?}", result);
}
