use std::error::Error;
use std::process::Command;

pub trait Gpg {
    fn fingerprints(&self) -> Result<Vec<String>, Box<Error>>;
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
            .map(|s| String::from(s.split(':').nth(9).unwrap()))
            .collect::<Vec<_>>();

        Ok(per_line)
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
    }

    #[test]
    fn list_fingerprints() {
        let result = (MockGpg{}).fingerprints().unwrap();
        assert_eq!(vec!(String::from("FF4666522286636A9dfge31AE5572467777449DBF6")), result);
    }
}