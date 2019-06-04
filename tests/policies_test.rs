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

#[test]
fn verify_git_commits_happy_path_from_empty() {
    set_current_dir_to_test_repo();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "0000000000000000000000000000000000000000", "69841d34d7dbef6d70ea9f59419c9fed7749575f","master");
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_happy_path_from_existing() {
    set_current_dir_to_test_repo();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "69841d34d7dbef6d70ea9f59419c9fed7749575f", "d4fd2666752b521da43735d64700f5a99329a126","master");
    assert!(result.is_ok(), "Error: {:?}", result);
}

#[test]
fn verify_git_commits_single_unsigned_commit() {
    set_current_dir_to_test_repo();
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>(&verify_commits_config(), "d4fd2666752b521da43735d64700f5a99329a126", "be693bc8ea72bf161e1003480eacf5cc4dcc23cd","master");
    assert!(result.is_err());
}
