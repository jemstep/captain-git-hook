use capn::gpg::*;
use std::collections::HashSet;

#[test]
fn call_for_fingerprints_completes_successfully() {
    let fingerprintids = LiveGpg::fingerprints();
    println!("{:?}", fingerprintids);
    assert!(fingerprintids.is_ok());
}

#[test]
#[ignore = "This test takes a long time to run"]
fn receive_keys() {
    let mut fingerprints = HashSet::new();
    fingerprints.insert("1212121212121212112".to_string());
    let result = LiveGpg::receive_keys("keyserver", &fingerprints);
    println!("Status {:?}", result);
    assert!(result.is_ok());
}
