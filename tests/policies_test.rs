use capn::config::{Config, VerifyGitCommitsConfig};
use capn::policies;
use capn;

use capn::git::LiveGit;
use capn::gpg::LiveGpg;

use std::process::*;


fn init_logging() {
    use log::LevelFilter;
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(LevelFilter::Trace)
        .try_init();
}

fn set_current_dir_to_test_repo() {
    let project_root = env!("CARGO_MANIFEST_DIR");
    std::env::set_current_dir(format!("{}/tests/test-repo.git", project_root)).unwrap();
}

fn import_test_key() {
    let project_root = env!("CARGO_MANIFEST_DIR");
    let status = Command::new("gpg")
        .args(&["--import", &format!("{}/tests/test-public-key.asc", project_root)])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
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
fn verify_git_commits_happy_path_from_empty_through_pre_receive() {
    init_logging();
    set_current_dir_to_test_repo();
    import_test_key();
    let config = Config {
        prepend_branch_name: None,
        verify_git_commits: Some(verify_commits_config())
    };
    let result = capn::pre_receive::<LiveGit, LiveGpg>(&config, "0000000000000000000000000000000000000000", "7f9763e189ade34345e683ab7e0c22d164280452", "master").unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_happy_path_from_empty() {
    init_logging();
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "0000000000000000000000000000000000000000", "7f9763e189ade34345e683ab7e0c22d164280452").unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_happy_path_from_existing() {
    init_logging();
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "7f9763e189ade34345e683ab7e0c22d164280452", "eb5e0185546b0bb1a13feec6b9ee8b39985fea42").unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_happy_path_unsigned_trivial_no_fast_forward_merge() {
    init_logging();
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "3eb315d10e2ad89555d7bfc78a1db1ce07bce434").unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_happy_path_unsigned_trivial_merge() {
    init_logging();
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "6754e4ec9b2dec567190d5a7f0be18b1a23d632a").unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_single_unsigned_commit() {
    init_logging();
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "d2e3bfdc923986d04e7a6368b5fdd78b1ddf84f1").unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_single_unsigned_commit_new_branch() {
    init_logging();
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "0000000000000000000000000000000000000000", "d2e3bfdc923986d04e7a6368b5fdd78b1ddf84f1").unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_unsigned_commit_being_merged_in() {
    init_logging();
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "ef1710ba8bd1f5ed0eec7883af30fca732d39afd").unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_unsigned_commit_behind_a_merge_commit() {
    init_logging();
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "e9752e78505f3c9bcec15d4bef4299caf0538388").unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_invalid_author() {
    init_logging();
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "afe2141ef20abd098927adc66d6728821cb34f59").unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_code_injected_into_unsigned_merge() {
    init_logging();
    set_current_dir_to_test_repo();
    import_test_key();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "eb5e0185546b0bb1a13feec6b9ee8b39985fea42", "eef93e7f977c125f92fc78116fc9b881e4055ae8").unwrap();
    assert!(result.is_err());
}
