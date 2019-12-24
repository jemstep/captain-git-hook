use capn::gpg::*;
use std::collections::HashSet;

#[test]
fn receive_keys() {
    let mut fingerprints = HashSet::new();
    fingerprints.insert("1212121212121212112".to_string());
    let result = LiveGpg {
        parallel_fetch: true,
        keyserver: "keyserver".to_string(),
    }
    .receive_keys(&fingerprints);

    // This key is made up, and the keyserver isn't valid, so this won't pass ever
    assert!(result.is_err());
}
