use git2::Repository;

/// Uses libgit to get the name of your current branch
pub fn get_current_branch() -> Result<String, git2::Error> {
    let git_repo = Repository::discover("./")?;
    let head = git_repo.head()?;
    let head_name =  head.shorthand();
    match head_name {
        Some(name) => Ok(name.to_string()),
        None => Err(git2::Error::from_str("No branch name found"))
    }
}
