extern crate capn;

use capn::gpg::*;

#[test]
fn fingerprints_exist() {
    let fingerprintids = (LiveGpg{}).fingerprints();
    println!("{:?}", fingerprintids);
    assert!(fingerprintids.unwrap().len() > 0);
}

#[test]
#[ignore("This test takes a long time to run")]
fn receive_keys() {
    let fingerprints = ["111111111111111111111111111111111111111111".to_string(), "111111111111111111111111111111111111111111".to_string()];
    let result = (LiveGpg{}).receive_keys("keyserver",&fingerprints);
    //let exitStatus = result.map;

    println!("Status {:?}", result);
    assert!(result.is_ok());
}
