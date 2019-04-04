use std::error::Error;
use std::process::*;

use crate::config::*;

pub trait Gpg {
    fn fingerprints(&self) -> Result<Vec<String>, Box<Error>>;
    fn receive_keys(&self, key_server: &str, fingerprints: &Vec<String>) -> Result<Option<ExitStatus>, Box<Error>>;
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

     fn receive_keys(&self, key_server: &str, fingerprints: &Vec<String>) -> Result<Option<ExitStatus>, Box<Error>> {
        let result = Command::new("gpg")
            .arg(format!("{} {}","--keyserver", key_server))
            .arg("--recv-keys ")
            .status()?;

        Ok(Some(result))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub struct MockGpg {}
    impl Gpg for MockGpg {
        fn fingerprints(&self) -> Result<Vec<String>, Box<Error>> {
            Ok(vec!(String::from("FF4666522286636A9dfge31AE5572467777449DBF6")))
        }
        fn receive_keys(&self, key_server: &str, fingerprints: &Vec<String>) -> Result<Option<ExitStatus>, Box<Error>> {
            Ok(None)
        }
    }

    #[test]
    fn list_fingerprints() {
        let result = (MockGpg{}).fingerprints().unwrap();
        assert_eq!(vec!(String::from("FF4666522286636A9dfge31AE5572467777449DBF6")), result);
    }
}