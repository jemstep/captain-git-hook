use std::error::Error;
use std::process::Command;

pub trait Gpg {
    fn fingerprints() -> Result<String, Box<Error>>;
}

pub struct GpgServer {}

impl Gpg for GpgServer {
    fn fingerprints() -> Result<String, Box<Error>> {
        let result = Command::new("gpg")
        .arg("--fingerprint")
        .output()
        .expect("failed to execute gpg");
        let encoded = String::from_utf8(result.stdout).unwrap();
        Ok(encoded)
    }
}

pub fn fingerprints<G: Gpg>() -> Result<String, Box<Error>> {
    let result = G::fingerprints()?;
    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;

    pub struct MockGpg {}
    impl Gpg for MockGpg {
        fn fingerprints() -> Result<String, Box<Error>> {
            Ok(String::from(""))
        }
    }

    #[test]
    fn list_fingerprints() {
        let result = fingerprints::<MockGpg>().unwrap();
        assert_eq!(String::from(""), result);
    }
}