use std::error::Error;
use std::process::*;
use crate::error::CapnError;

use rayon::prelude::*;
use log::*;

pub trait Gpg {
    fn fingerprints() -> Result<Vec<String>, Box<dyn Error>>;
    fn receive_keys(key_server: &str, fingerprints: &[String]) -> Result<(), Box<dyn Error>>;
    fn par_receive_keys(key_server: &str, fingerprints: &[String]) -> Result<(), Box<dyn Error>>;
}

pub struct LiveGpg {}

impl Gpg for LiveGpg {
    fn fingerprints() -> Result<Vec<String>, Box<dyn Error>> {
        let result = Command::new("gpg")
            .arg("--with-colons")
            .arg("--fingerprint")
            .output()?;
        if !result.status.success() {
            warn!("Call to GPG failed");
            return Err(Box::new(CapnError::new(format!("Call to GPG failed with staus {}", result.status))));
        }

        let encoded = String::from_utf8(result.stdout)?;
        let per_line = encoded.split('\n')
            .filter(|s| s.starts_with("fpr"))
            .filter_map(|s| s.split(':').nth(9).map(String::from))
            .collect::<Vec<_>>();

        Ok(per_line)
    }

    fn receive_keys(key_server: &str, fingerprints: &[String]) -> Result<(), Box<dyn Error>> {
        trace!("Receiving keys for fingerprints {:?}",fingerprints);
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

    fn par_receive_keys(key_server: &str, fingerprints: &[String]) -> Result<(), Box<dyn Error>> {
        trace!("Receiving keys for fingerprints in PARALLEL");
        let _r : Vec<_> = fingerprints.par_iter().map(|fp| {
            match Self::receive_keys(&key_server, &[fp.to_string()]){
                Ok(o) => o,
                Err(e) => error!("Error receiving key for {} : {}", fp, e)
            };
            }
            ).collect();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub struct MockGpg {}
    impl Gpg for MockGpg {
        fn fingerprints() -> Result<Vec<String>, Box<dyn Error>> {
            Ok(vec!(String::from("111111111111111111111111111111111111111111")))
        }
        fn receive_keys(_key_server: &str, _fingerprints: &[String]) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
        fn par_receive_keys(_key_server: &str, _fingerprints: &[String]) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    #[test]
    fn list_fingerprints() {
        let result = MockGpg::fingerprints().unwrap();
        assert_eq!(vec!(String::from("111111111111111111111111111111111111111111")), result);
    }
}
