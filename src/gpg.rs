use std::error::Error;
use std::process::Command;

pub trait Gpg {
    fn fingerprints() -> Result<Vec<String>, Box<Error>>;
}

pub struct GpgServer {}

impl Gpg for GpgServer {
    fn fingerprints() -> Result<Vec<String>, Box<Error>> {
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

pub fn fingerprints<G: Gpg>() -> Result<Vec<String>, Box<Error>> {
    G::fingerprints()    
}

#[cfg(test)]
mod test {
    use super::*;

    pub struct MockGpg {}
    impl Gpg for MockGpg {
        fn fingerprints() -> Result<Vec<String>, Box<Error>> {
            Ok(vec!(String::from("")))
        }
    }

    #[test]
    fn list_fingerprints() {
        let result = fingerprints::<MockGpg>().unwrap();
        assert_eq!(vec!(String::from("")), result);
    }
}