use crate::local_git::git;
use std::path::Path;

pub fn checkout(root: &Path, branch: &str) -> Result<(), String> {
    git(root, &["switch", branch]).map(|_| ())
}

fn has_upstream(root: &Path) -> bool {
    git(
        root,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .is_ok()
}

pub fn push(root: &Path, branch: &str) -> Result<String, String> {
    if has_upstream(root) {
        git(root, &["push"])?;
    } else {
        git(root, &["push", "-u", "origin", branch])?;
    }
    Ok("推送好啦喵~".to_string())
}
