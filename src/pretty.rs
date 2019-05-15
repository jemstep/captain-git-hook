

const SEPERATOR: &str = "********************************************************************************************";

pub fn seperator(text: &str) -> String {
    format!("{}\n{}\n", text, SEPERATOR)
}

pub fn block(text: &str) -> String {
    format!("\n{}\n{}\n{}\n", SEPERATOR, text, SEPERATOR)
}