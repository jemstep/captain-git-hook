extern crate capn;

use capn::policies::*;

#[test]
//#[ignore("This test takes a long time to run")]
fn verify_git_commits() {
    let result = capn::policies::verify_git_commits("new_value","gpg/TEAM_FINGERPRINTS","keyserver0");
    assert!(result.is_ok());
}
