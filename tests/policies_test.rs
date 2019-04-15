use capn::policies;
use capn::git::LiveGit;
use capn::gpg::LiveGpg;

#[test]
#[ignore("This functionality has not been implemented yet")]
fn verify_git_commits() {
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>("new_value","gpg/TEAM_FINGERPRINTS","keyserver0", true);
    assert!(result.is_ok());
}
