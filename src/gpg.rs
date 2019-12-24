use crate::error::CapnError;
use std::collections::HashSet;
use std::error::Error;
use std::process::*;

use log::*;
use rayon::prelude::*;

pub trait Gpg {
    fn receive_keys(&self, fingerprints: &HashSet<String>) -> Result<(), Box<dyn Error>>;
}

pub struct LiveGpg {
    pub parallel_fetch: bool,
    pub keyserver: String,
}

impl Gpg for LiveGpg {
    fn receive_keys(&self, fingerprints: &HashSet<String>) -> Result<(), Box<dyn Error>> {
        if self.parallel_fetch {
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
        }
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
        fn receive_keys(&self, _fingerprints: &HashSet<String>) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }
}
