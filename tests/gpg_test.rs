extern crate capn;

use capn::gpg::*;

#[test]
fn it_works() {
    println!("{:?}", fingerprints::<GpgServer>());
    assert_eq!(1, 1);
}
