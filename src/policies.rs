use crate::git::*;

use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::io::prelude::*;

pub fn prepend_branch_name(commit_file: PathBuf) -> Result<(), Box<Error>> {
    let branch = get_current_branch()?;
    Ok(prepend_string_to_file(branch, commit_file)?)
}


fn prepend_string_to_file(s: String, filename: PathBuf) -> Result<(), std::io::Error> {
    // It turns out that prepending a string to a file is not an
    // obvious action. You can only write to the end of a file :(
    //
    // The solution is to read the existing contents, then write a new
    // file starting with the branch name, and then writing the rest
    // of the file.

    let mut read_file = File::open(&filename)?;
    let mut current_contents = String::new();
    read_file.read_to_string(&mut current_contents)?;

    let mut write_file = File::create(&filename)?;

    writeln!(write_file, "{}:", s)?;
    write!(write_file, "{}", current_contents)
}
