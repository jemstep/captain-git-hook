use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct CapnError {
    pub reason: String,
}

impl CapnError {
    pub fn new(reason: String) -> CapnError {
        CapnError { reason }
    }
}

impl fmt::Display for CapnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl Error for CapnError {}
