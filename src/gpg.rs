use crate::error::CapnError;
use crate::keyring::Keyring;
use std::collections::HashSet;
use std::error::Error;
use std::process::*;
use std::time::Instant;

use log::*;
use rayon::prelude::*;

pub trait Gpg {
    fn receive_keys(
        &self,
        keyring: &mut Keyring,
        emails: &HashSet<String>,
    ) -> Result<(), Box<dyn Error>>;
}

pub struct LiveGpg {
    pub parallel_fetch: bool,
    pub keyserver: String,
}

impl Gpg for LiveGpg {
    fn receive_keys(
        &self,
        keyring: &mut Keyring,
        emails: &HashSet<String>,
    ) -> Result<(), Box<dyn Error>> {
        let start = Instant::now();

        let fingerprints: Vec<String> = emails
            .iter()
            .filter(|email| {
                keyring
                    .fingerprints
                    .get(*email)
                    .map(|f| f.pubkey_downloaded)
                    == Some(false)
            })
            .filter_map(|email| keyring.fingerprint_id_from_email(email))
            .collect();

        let fetch_result = if self.parallel_fetch {
            if fingerprints
                .par_iter()
                .map(|fp| match self.receive_key(fp) {
                    Ok(_) => true,
                    Err(e) => {
                        error!("Error receiving key for {} : {}", fp, e);
                        false
                    }
                })
                .all(|success| success)
            {
                Ok(())
            } else {
                Err(Box::new(CapnError::new(
                    "Error fetching GPG key".to_string(),
                )))
            }
        } else {
            let result = Command::new("gpg")
                .args(&["--keyserver", &self.keyserver])
                .arg("--recv-keys")
                .args(fingerprints)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;

            if result.success() {
                Ok(())
            } else {
                Err(Box::new(CapnError::new(format!(
                    "Call to GPG keyserver failed with code {:?}",
                    result.code()
                ))))
            }
        };

        if let Ok(_) = fetch_result {
            for email in emails {
                keyring
                    .fingerprints
                    .get_mut(email)
                    .map(|f| f.pubkey_downloaded = true);
            }
        }
        fetch_result?;

        trace!(
            "GPG receive_keys completed in: {} ms",
            start.elapsed().as_millis()
        );

        Ok(())
    }
}

impl LiveGpg {
    fn receive_key(&self, fingerprint: &str) -> Result<(), Box<dyn Error>> {
        debug!("Receiving key for fingerprint {:?}", fingerprint);

        let result = Command::new("gpg")
            .args(&["--keyserver", &self.keyserver])
            .arg("--recv-keys")
            .arg(fingerprint)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if result.success() {
            Ok(())
        } else {
            Err(Box::new(CapnError::new(format!(
                "Call to GPG keyserver failed with code {:?}",
                result.code()
            ))))
        }
    }
}

pub mod test {
    use super::*;

    pub struct MockGpg;
    impl Gpg for MockGpg {
        fn receive_keys(
            &self,
            keyring: &mut Keyring,
            emails: &HashSet<String>,
        ) -> Result<(), Box<dyn Error>> {
            for email in emails {
                keyring
                    .fingerprints
                    .get_mut(email)
                    .map(|f| f.pubkey_downloaded = true);
            }

            Ok(())
        }
    }
}
