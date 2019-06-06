use capn::config::VerifyGitCommitsConfig;
use capn::policies;
use capn::git::LiveGit;
use capn::gpg::LiveGpg;

use std::process::*;


fn init_logging() {
    let _ = stderrlog::new()
        .verbosity(5)
        .init();
}

fn set_current_dir_to_test_repo() {
    let project_root = env!("CARGO_MANIFEST_DIR");
    std::env::set_current_dir(format!("{}/tests/test-repo.git", project_root)).unwrap();
}

fn import_test_key() {
    let project_root = env!("CARGO_MANIFEST_DIR");
    let status = Command::new("gpg")
        .args(&["--import", &format!("{}/tests/test-public-key.asc", project_root)])
        .status()
        .unwrap();
    assert!(status.success(), "Failed to import test GPG key");
}

fn verify_commits_config() -> VerifyGitCommitsConfig {
    VerifyGitCommitsConfig {
        author_domain : "jemstep.com".to_string(), 
        committer_domain : "jemstep.com".to_string(),
        keyserver : "hkp://p80.pool.sks-keyservers.net".to_string(),
        team_fingerprints_file: "TEAM_FINGERPRINTS".to_string(),
        recv_keys_par : true,
        skip_recv_keys: true,
        verify_email_addresses: true,
        verify_commit_signatures: true,
        verify_different_authors: true
    }
}

#[test]
fn verify_git_commits_happy_path_from_empty() {
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "0000000000000000000000000000000000000000", "7f9763e189ade34345e683ab7e0c22d164280452", "master");
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_happy_path_from_existing() {
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "7f9763e189ade34345e683ab7e0c22d164280452", "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "master");
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_single_unsigned_commit() {
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "d2e3bfdc923986d04e7a6368b5fdd78b1ddf84f1", "master");
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_single_unsigned_commit_new_branch() {
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "0000000000000000000000000000000000000000", "d2e3bfdc923986d04e7a6368b5fdd78b1ddf84f1", "unsigned");
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_unsigned_commit_being_merged_in() {
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "ef1710ba8bd1f5ed0eec7883af30fca732d39afd", "master");
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_unsigned_commit_behind_a_merge_commit() {
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "e9752e78505f3c9bcec15d4bef4299caf0538388", "master");
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_invalid_author() {
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "afe2141ef20abd098927adc66d6728821cb34f59", "master");
    assert!(result.is_err());
}
