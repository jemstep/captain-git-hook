use std::collections::HashMap;

pub struct Keyring {
    pub fingerprints: HashMap<String, Fingerprint>,
}

pub struct Fingerprint {
    pub id: String,
    pub name: String,
    pub email: String,
    pub pubkey_downloaded: bool,
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
                                pubkey_downloaded: false,
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
}
