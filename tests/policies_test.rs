use capn::policies;
use capn::git::LiveGit;
use capn::gpg::LiveGpg;

#[test]
#[ignore("This functionality has not been implemented yet")]
fn verify_git_commits() {
    let result = policies::verify_git_commits::<LiveGit, LiveGpg>("old_value","new_value","ref_name","gpg/TEAM_FINGERPRINTS","keyserver0", true);
    assert!(result.is_ok());
}
