use crate::local_git::git;
use std::path::Path;

pub fn stage(root: &Path, path: &str) -> Result<(), String> {
    git(root, &["add", "--", path]).map(|_| ())
}

pub fn unstage(root: &Path, path: &str) -> Result<(), String> {
    if git(root, &["restore", "--staged", "--", path]).is_ok() {
        return Ok(());
    }
    git(root, &["rm", "--cached", "--quiet", "--", path]).map(|_| ())
}

pub fn stage_all(root: &Path) -> Result<(), String> {
    git(root, &["add", "-A"]).map(|_| ())
}

pub fn unstage_all(root: &Path) -> Result<(), String> {
    if git(root, &["restore", "--staged", "."]).is_ok() {
        return Ok(());
    }
    git(root, &["rm", "--cached", "-r", "--quiet", "."]).map(|_| ())
}

pub fn commit(root: &Path, message: &str) -> Result<(), String> {
    git(root, &["commit", "-m", message]).map(|_| ())
}

pub fn amend(root: &Path, message: &str) -> Result<(), String> {
    git(root, &["commit", "--amend", "-m", message]).map(|_| ())
}

pub fn create_branch(root: &Path, name: &str) -> Result<(), String> {
    git(root, &["switch", "-c", name]).map(|_| ())
}

pub fn discard(root: &Path, path: &str) -> Result<(), String> {
    if git(root, &["ls-files", "--error-unmatch", "--", path]).is_ok() {
        git(
            root,
            &[
                "restore",
                "--source=HEAD",
                "--staged",
                "--worktree",
                "--",
                path,
            ],
        )
        .map(|_| ())
    } else {
        git(root, &["clean", "-f", "--", path]).map(|_| ())
    }
}

pub fn stash_save(root: &Path) -> Result<String, String> {
    let output = git(
        root,
        &["stash", "push", "--include-untracked", "-m", "gxt stash"],
    )?;
    if output.contains("No local changes") {
        return Err("没有可以 stash 的改动喵~".to_string());
    }
    Ok("改动 stash 起来啦喵~".to_string())
}

pub fn stash_pop(root: &Path) -> Result<String, String> {
    git(root, &["stash", "pop"])?;
    Ok("stash 恢复好啦喵~".to_string())
}

pub fn checkout(root: &Path, branch: &str) -> Result<(), String> {
    git(root, &["switch", branch]).map(|_| ())
}

pub fn create_tag(root: &Path, name: &str) -> Result<(), String> {
    git(root, &["tag", name]).map(|_| ())
}

pub fn file_diff(root: &Path, path: &str, staged: bool) -> Result<String, String> {
    let args = if staged {
        vec!["diff", "--no-color", "--cached", "--", path]
    } else {
        vec!["diff", "--no-color", "--", path]
    };
    let diff = git(root, &args)?;
    if !diff.trim().is_empty() {
        return Ok(diff);
    }
    if !staged {
        let full = root.join(path);
        if full.is_file()
            && let Ok(text) = std::fs::read_to_string(&full)
        {
            let mut out = format!("--- /dev/null\n+++ b/{path}\n@@ 未跟踪文件 @@\n");
            for line in text.lines().take(600) {
                out.push('+');
                out.push_str(line);
                out.push('\n');
            }
            return Ok(out);
        }
    }
    Ok(String::new())
}

pub fn commit_diff(root: &Path, hash: &str) -> Result<String, String> {
    let diff = git(
        root,
        &[
            "show",
            "--no-color",
            "--stat",
            "--patch",
            "--format=fuller",
            hash,
        ],
    )?;
    Ok(limit_lines(diff, 2_000))
}

pub fn ref_log(root: &Path, reference: &str) -> Result<String, String> {
    let log = git(
        root,
        &["log", "--no-color", "--oneline", "-n", "30", reference],
    )?;
    Ok(limit_lines(log, 400))
}

pub fn tag_info(root: &Path, tag: &str) -> Result<String, String> {
    let info = git(
        root,
        &["show", "--no-color", "--stat", "--format=fuller", tag],
    )?;
    Ok(limit_lines(info, 400))
}

pub fn push(root: &Path, branch: &str) -> Result<String, String> {
    let has_upstream = git(
        root,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .is_ok();
    if has_upstream {
        git(root, &["push"])?;
    } else {
        git(root, &["push", "-u", "origin", branch])?;
    }
    Ok("推送好啦喵~".to_string())
}

pub fn push_tags(root: &Path) -> Result<String, String> {
    git(root, &["push", "--tags"])?;
    Ok("标签都推上去啦喵~".to_string())
}

pub fn fetch(root: &Path) -> Result<String, String> {
    git(root, &["fetch", "--all", "--prune"])?;
    Ok("远端更新取回来啦喵~".to_string())
}

pub fn pull(root: &Path) -> Result<String, String> {
    git(root, &["pull", "--ff-only"])?;
    Ok("拉取好啦喵~".to_string())
}

fn limit_lines(text: String, limit: usize) -> String {
    let mut out = String::new();
    for (index, line) in text.lines().enumerate() {
        if index >= limit {
            out.push_str("... 后面还有更多内容喵~\n");
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}
