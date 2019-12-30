use capn::gpg::*;
use capn::keyring::{Fingerprint, Keyring};
use std::collections::{HashMap, HashSet};

#[test]
fn receive_keys_attempts_to_fetch_unfetched_keys() {
    let mut keyring = Keyring {
        fingerprints: HashMap::new(),
    };
    keyring.fingerprints.insert(
        "test@jemstep.com".to_string(),
        Fingerprint {
            id: "1212121212121212112".to_string(),
            name: "Test User".to_string(),
            email: "test@jemstep.com".to_string(),
            pubkey_downloaded: false,
        },
    );

    let mut emails = HashSet::new();
    emails.insert("test@jemstep.com".to_string());

    let result = LiveGpg {
        parallel_fetch: true,
        keyserver: "keyserver".to_string(),
    }
    .receive_keys(&mut keyring, &emails);

    // This key is made up, and the keyserver isn't valid, so this won't pass ever
    assert!(result.is_err());
}

#[test]
fn receive_keys_does_not_fetch_already_fetched_keys() {
    let mut keyring = Keyring {
        fingerprints: HashMap::new(),
    };
    keyring.fingerprints.insert(
        "test@jemstep.com".to_string(),
        Fingerprint {
            id: "1212121212121212112".to_string(),
            name: "Test User".to_string(),
            email: "test@jemstep.com".to_string(),
            pubkey_downloaded: true,
        },
    );

    let mut emails = HashSet::new();
    emails.insert("test@jemstep.com".to_string());

    let result = LiveGpg {
        parallel_fetch: true,
        keyserver: "keyserver".to_string(),
    }
    .receive_keys(&mut keyring, &emails);

    // This key is made up, so this is successful only if there was no request made
    assert!(result.is_ok());
}
