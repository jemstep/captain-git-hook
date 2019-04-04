extern crate capn;

use capn::gpg::*;

#[test]
fn fingerprints_exist() {
    let fingerprintids = (LiveGpg{}).fingerprints();
    println!("{:?}", fingerprintids);
    assert!(fingerprintids.unwrap().len() > 0);
}
