use std::error::Error;
use std::process::*;
use crate::error::CapnError;

use log::*;

pub trait Gpg {
    fn fingerprints(&self) -> Result<Vec<String>, Box<Error>>;
    fn receive_keys(&self, key_server: &str, fingerprints: &[String]) -> Result<(), Box<Error>>;
}

pub struct LiveGpg {}

impl Gpg for LiveGpg {
    fn fingerprints(&self) -> Result<Vec<String>, Box<Error>> {
        let result = Command::new("gpg")
            .arg("--with-colons")
            .arg("--fingerprint")
            .output()?;
        let encoded = String::from_utf8(result.stdout)?;
        let per_line = encoded.split('\n')
            .filter(|s| s.starts_with("fpr"))
            .filter_map(|s| s.split(':').nth(9).map(String::from))
            .collect::<Vec<_>>();

        Ok(per_line)
    }

     fn receive_keys(&self, key_server: &str, fingerprints: &[String]) -> Result<(), Box<Error>> {
        trace!("Fingerprints {:?}",fingerprints);
        let result = Command::new("gpg")
            .args(&["--keyserver",key_server])
            .arg("--recv-keys")
            .args(fingerprints)
            .status()?;

            if result.success() {
                Ok(())
            } else {
                Err(Box::new(CapnError::new(format!("Call to GPG keyserver failed with code {:?}", result.code()))))
            }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub struct MockGpg {}
    impl Gpg for MockGpg {
        fn fingerprints(&self) -> Result<Vec<String>, Box<Error>> {
            Ok(vec!(String::from("111111111111111111111111111111111111111111")))
        }
        fn receive_keys(&self, key_server: &str, fingerprints: &[String]) -> Result<(), Box<Error>> {
            Ok(())
        }
    }

    #[test]
    fn list_fingerprints() {
        let result = (MockGpg{}).fingerprints().unwrap();
        assert_eq!(vec!(String::from("111111111111111111111111111111111111111111")), result);
    }
}
