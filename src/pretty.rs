use std::fmt::Display;

const SEPERATOR: &str = "********************************************************************************************";

pub fn seperator(text: impl Display) -> String {
    format!("{}\n{}\n", text, SEPERATOR)
}

pub fn block(text: impl Display) -> String {
    format!("\n{0}\n{1}\n{0}\n", SEPERATOR, text)
}
