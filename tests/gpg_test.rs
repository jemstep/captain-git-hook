use capn::gpg::*;

#[test]
fn call_for_fingerprints_completes_successfully() {
    let fingerprintids = (LiveGpg{}).fingerprints();
    println!("{:?}", fingerprintids);
    assert!(fingerprintids.is_ok());
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
