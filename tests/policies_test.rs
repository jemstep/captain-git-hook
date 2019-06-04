use capn::config::VerifyGitCommitsConfig;
use capn::policies;
use capn::git::LiveGit;
use capn::gpg::LiveGpg;

fn set_current_dir_to_test_repo() {
    let project_root = env!("CARGO_MANIFEST_DIR");
    println!("Project root: {}", project_root);
    println!("Current dir: {:?}", std::env::current_dir());
    std::env::set_current_dir(format!("{}/tests/test-repo.git", project_root)).unwrap();
}

fn verify_commits_config() -> VerifyGitCommitsConfig {
    VerifyGitCommitsConfig {
        author_domain : "jemstep.com".to_string(), 
        committer_domain : "jemstep.com".to_string(),
        keyserver : "hkp://p80.pool.sks-keyservers.net".to_string(),
        team_fingerprints_file: "TEAM_FINGERPRINTS".to_string(),
        recv_keys_par : true,
        verify_email_addresses: true,
        verify_commit_signatures: true,
        verify_different_authors: true
    }
}

/// Many of these tests require a test git repo in a particular
/// state. This is a bare git repo, checked into this repo
/// (./test-repo.git). To add extra scenarios, you'll probably need to
/// add additional commits to this bare repo. To do this, clone the
/// repo somewhere else on your drive with `git clone <path to
/// test-repo.git>`, make any required commits, and push the changes
/// back. Then commit the changes in this repo.
///
/// To create valid commits for these tests, you need to sign the
/// commits with the secret key in ./test-secret-key.asc. The password
/// to import this key is 'test'.

#[test]
fn verify_git_commits_happy_path_from_empty() {
    set_current_dir_to_test_repo();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "0000000000000000000000000000000000000000", "7f9763e189ade34345e683ab7e0c22d164280452","master");
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_happy_path_from_existing() {
    set_current_dir_to_test_repo();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "7f9763e189ade34345e683ab7e0c22d164280452", "eb5e0185546b0bb1a13feec6b9ee8b39985fea42","master");
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_single_unsigned_commit() {
    set_current_dir_to_test_repo();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "d2e3bfdc923986d04e7a6368b5fdd78b1ddf84f1","master");
    assert!(result.is_err());
}
