use std::collections::{HashMap, HashSet};

pub struct Keyring {
    pub fingerprints: HashMap<String, Fingerprint>,
}

pub struct Fingerprint {
    pub id: String,
    pub name: String,
    pub email: String,
    pub public_key_is_available_locally: bool,
}

impl Keyring {
    pub fn from_team_fingerprints_file(file_contents: String) -> Keyring {
        let fingerprints: HashMap<String, Fingerprint> = file_contents
            .split('\n')
            .filter_map(|l| {
                let line: Vec<&str> = l.split(',').collect();
                match &line[..] {
                    [fingerprint, name, email] => {
                        let fingerprint = fingerprint.replace(char::is_whitespace, "");
                        Some((
                            email.to_string(),
                            Fingerprint {
                                id: fingerprint,
                                name: name.to_string(),
                                email: email.to_string(),
                                public_key_is_available_locally: false,
                            },
                        ))
                    }
                    _ => None,
                }
            })
            .collect();
        Keyring { fingerprints }
    }

    pub fn fingerprint_id_from_email(&self, email: &str) -> Option<String> {
        self.fingerprints.get(email).map(|f| f.id.clone())
    }

    pub fn requires_public_key_download(&self, email: &str) -> bool {
        self.fingerprints
            .get(email)
            .filter(|f| !f.public_key_is_available_locally)
            .is_some()
    }

    pub fn mark_public_keys_available(&mut self, emails: &HashSet<&str>) {
        for email in emails {
            self.fingerprints
                .get_mut(&email.to_string())
                .map(|f| f.public_key_is_available_locally = true);
        }
    }
}
