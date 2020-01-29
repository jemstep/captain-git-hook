use capn;
use capn::config::{Config, GitConfig, VerifyGitCommitsConfig};
use capn::policies;

use capn::git::LiveGit;
use capn::gpg::test::MockGpg;

use capn::logger::Logger;

use std::process::*;

use std::sync::Once;

static BEFORE_ALL: Once = Once::new();

fn before_all() {
    BEFORE_ALL.call_once(|| {
        init_logging();
        set_current_dir_to_test_repo();
        import_test_key();
    });
}

fn init_logging() {
    Logger::test_init();
}

fn set_current_dir_to_test_repo() {
    let project_root = env!("CARGO_MANIFEST_DIR");
    std::env::set_current_dir(format!("{}/tests/test-repo.git", project_root)).unwrap();
}

fn import_test_key() {
    let project_root = env!("CARGO_MANIFEST_DIR");
    let status = Command::new("gpg")
        .args(&[
            "--import",
            &format!("{}/tests/test-public-key.asc", project_root),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "Failed to import test GPG key");
}

fn verify_commits_config() -> VerifyGitCommitsConfig {
    VerifyGitCommitsConfig {
        author_domain: "jemstep.com".to_string(),
        committer_domain: "jemstep.com".to_string(),
        keyserver: "hkp://p80.pool.sks-keyservers.net".to_string(),
        team_fingerprints_file: "TEAM_FINGERPRINTS".to_string(),
        recv_keys_par: true,
        verify_email_addresses: true,
        verify_commit_signatures: true,
        verify_different_authors: true,
        override_tag_pattern: Some("capn-override-*".to_string()),
        override_tags_required: 1,
    }
}

#[test]
fn verify_git_commits_happy_path_from_empty_through_pre_receive() {
    before_all();
    let config = Config {
        git: GitConfig::default(),
        prepend_branch_name: None,
        verify_git_commits: Some(verify_commits_config()),
    };
    let result = capn::pre_receive::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &config,
        "0000000000000000000000000000000000000000",
        "7f9763e189ade34345e683ab7e0c22d164280452",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_happy_path_from_empty() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "0000000000000000000000000000000000000000",
        "7f9763e189ade34345e683ab7e0c22d164280452",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_happy_path_from_existing() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "7f9763e189ade34345e683ab7e0c22d164280452",
        "eb5e0185546b0bb1a13feec6b9ee8b39985fea42",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_happy_path_unsigned_trivial_no_fast_forward_merge() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "eb5e0185546b0bb1a13feec6b9ee8b39985fea42",
        "3eb315d10e2ad89555d7bfc78a1db1ce07bce434",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_happy_path_unsigned_trivial_merge() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "eb5e0185546b0bb1a13feec6b9ee8b39985fea42",
        "6754e4ec9b2dec567190d5a7f0be18b1a23d632a",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_single_unsigned_commit() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "eb5e0185546b0bb1a13feec6b9ee8b39985fea42",
        "d2e3bfdc923986d04e7a6368b5fdd78b1ddf84f1",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_single_unsigned_commit_new_branch() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "0000000000000000000000000000000000000000",
        "d2e3bfdc923986d04e7a6368b5fdd78b1ddf84f1",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_unsigned_commit_being_merged_in() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "eb5e0185546b0bb1a13feec6b9ee8b39985fea42",
        "ef1710ba8bd1f5ed0eec7883af30fca732d39afd",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_unsigned_commit_behind_a_merge_commit() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "eb5e0185546b0bb1a13feec6b9ee8b39985fea42",
        "e9752e78505f3c9bcec15d4bef4299caf0538388",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_invalid_author() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "eb5e0185546b0bb1a13feec6b9ee8b39985fea42",
        "afe2141ef20abd098927adc66d6728821cb34f59",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_code_injected_into_unsigned_merge() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "eb5e0185546b0bb1a13feec6b9ee8b39985fea42",
        "eef93e7f977c125f92fc78116fc9b881e4055ae8",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_happy_path_pushing_previously_checked_merge_commit() {
    // This is an edge case for checking that merges have multiple authors
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "3eb315d10e2ad89555d7bfc78a1db1ce07bce434",
        "3eb315d10e2ad89555d7bfc78a1db1ce07bce434",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_author_merged_own_code_not_on_head() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "eb5e0185546b0bb1a13feec6b9ee8b39985fea42",
        "6004dfdb071c71e5e76ad55b924b576487e1c485",
        "refs/heads/valid-branch",
    )
    .unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_author_merged_own_code_on_configured_mainline() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::new(
            "./",
            GitConfig {
                mainlines: vec!["valid-*".into()],
            },
        )
        .unwrap(),
        MockGpg,
        &verify_commits_config(),
        "eb5e0185546b0bb1a13feec6b9ee8b39985fea42",
        "6004dfdb071c71e5e76ad55b924b576487e1c485",
        "refs/heads/valid-branch",
    )
    .unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_author_merged_own_code_on_head() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "eb5e0185546b0bb1a13feec6b9ee8b39985fea42",
        "6004dfdb071c71e5e76ad55b924b576487e1c485",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_err());
}

#[test]
fn verify_git_commits_author_merged_own_code_on_head_with_tag() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "eb5e0185546b0bb1a13feec6b9ee8b39985fea42",
        "e5924d0748c8852d74049679b34ca4b3b0570d0d",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_ok());
}

#[test]
fn verify_tagged_git_commits_override_rules() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &verify_commits_config(),
        "7f9763e189ade34345e683ab7e0c22d164280452",
        "6f00838625cd1b7dc0acc66e43fee5594f0f124c",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_tagged_git_commits_not_overridden_if_not_enough_tags() {
    before_all();
    let result = policies::verify_git_commits::<LiveGit, MockGpg>(
        &LiveGit::default("./").unwrap(),
        MockGpg,
        &VerifyGitCommitsConfig {
            override_tags_required: 2,
            ..verify_commits_config()
        },
        "7f9763e189ade34345e683ab7e0c22d164280452",
        "6f00838625cd1b7dc0acc66e43fee5594f0f124c",
        "refs/heads/master",
    )
    .unwrap();
    assert!(result.is_err());
}
