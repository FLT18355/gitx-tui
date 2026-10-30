use std::path::Path;
use std::process::Command;

pub fn git(path: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .map_err(|err| format!("跑不了 git 喵~：{err}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if detail.is_empty() {
            format!("git {} 没有成功喵~", args.join(" "))
        } else {
            detail
        })
    }
}
