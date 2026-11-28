use crate::local_git::git;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct StatusFile {
    pub path: String,
    pub index: char,
    pub worktree: char,
}

impl StatusFile {
    pub fn untracked(&self) -> bool {
        self.index == '?' && self.worktree == '?'
    }

    pub fn status_label(&self) -> String {
        if self.untracked() {
            "未跟踪".to_string()
        } else {
            let text = format!("{}{}", self.index, self.worktree);
            match text.trim() {
                "" => "无改动".to_string(),
                "M" | "MM" | " M" => "修改".to_string(),
                "A" | "AM" | " A" => "新增".to_string(),
                "D" | " D" => "删除".to_string(),
                "R" => "重命名".to_string(),
                "C" => "复制".to_string(),
                other => other.to_string(),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Commit {
    pub short: String,
    pub author: String,
    pub when: String,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub when: String,
    pub subject: String,
}

#[derive(Debug, Clone, Default)]
pub struct LocalRepo {
    pub root: PathBuf,
    pub branch: String,
    pub upstream: String,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<StatusFile>,
    pub commits: Vec<Commit>,
    pub tags: Vec<Tag>,
}

impl LocalRepo {
    pub fn open(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Err(format!("路径不存在喵~：{}", path.display()));
        }
        if !path.is_dir() {
            return Err(format!("这个路径不是目录喵~：{}", path.display()));
        }
        let root = match git(path, &["rev-parse", "--show-toplevel"]) {
            Ok(root) => root,
            Err(error) if error.starts_with("跑不了 git") => return Err(error),
            Err(_) => {
                return Err(format!("这里不是 git 仓库喵~：{}", path.display()));
            }
        };
        let root = PathBuf::from(root.trim());
        if root.as_os_str().is_empty() {
            return Err("这里不是 git 仓库喵~换一个路径试试吧喵~".to_string());
        }
        let status = git(&root, &["status", "--porcelain=v1", "--branch"])?;
        let (branch, upstream, ahead, behind, files) = parse_status(&status);
        let commits = git(
            &root,
            &["log", "-n", "100", "--pretty=format:%h%x1f%an%x1f%ar%x1f%s"],
        )
        .map(|raw| parse_commits(&raw))
        .unwrap_or_default();
        let tags = git(
            &root,
            &[
                "tag",
                "--sort=-creatordate",
                "--format=%(refname:short)\t%(creatordate:relative)\t%(subject)",
            ],
        )
        .map(|raw| parse_tags(&raw))
        .unwrap_or_default();
        Ok(Self {
            root,
            branch,
            upstream,
            ahead,
            behind,
            files,
            commits,
            tags,
        })
    }

    pub fn display_root(&self) -> String {
        crate::format::display_path(&self.root)
    }
}

#[derive(Debug, Clone, Default)]
pub struct LocalClone {
    pub path: PathBuf,
    pub name: String,
    pub branch: String,
    pub remote: String,
    pub full_name: Option<String>,
    pub dirty: usize,
    pub ahead: u32,
    pub behind: u32,
    pub checked: bool,
}

/// 扫一遍本地克隆目录，只收有 .git 的子目录喵~
pub fn scan_clones(root: &Path) -> Result<Vec<LocalClone>, String> {
    std::fs::create_dir_all(root).map_err(|error| format!("本地克隆目录建不出来：{error}喵~"))?;
    let entries =
        std::fs::read_dir(root).map_err(|error| format!("本地克隆目录读不了：{error}喵~"))?;
    let mut clones = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || !path.join(".git").exists() {
            continue;
        }
        if let Ok(clone) = read_clone(&path, false) {
            clones.push(clone);
        }
    }
    sort_clones(&mut clones);
    Ok(clones)
}

fn sort_clones(clones: &mut [LocalClone]) {
    clones.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
    });
}

fn read_clone(path: &Path, checked: bool) -> Result<LocalClone, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
        .to_string();
    let status = git(path, &["status", "--porcelain=v1", "--branch"])?;
    let (branch, _upstream, ahead, behind, files) = parse_status(&status);
    let remote = git(path, &["remote", "get-url", "origin"])
        .unwrap_or_default()
        .trim()
        .to_string();
    let full_name = github_full_name(&remote);
    Ok(LocalClone {
        path: path.to_path_buf(),
        name,
        branch,
        remote,
        full_name,
        dirty: files.len(),
        ahead,
        behind,
        checked,
    })
}

/// 检查更新：先 fetch，再读一次本地和远端的分叉情况喵~
pub fn check_clone(path: &Path) -> Result<LocalClone, String> {
    git(path, &["fetch", "--prune"])?;
    read_clone(path, true)
}

/// Pull 只用 fast-forward，遇到冲突就停下来让主人自己处理喵~
pub fn pull_clone(path: &Path) -> Result<LocalClone, String> {
    git(path, &["pull", "--ff-only"])?;
    read_clone(path, true)
}

/// 删除前确认目标真的在本地克隆目录里面，防止手滑删到别处喵！
pub fn delete_clone(root: &Path, path: &Path) -> Result<(), String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("本地克隆目录找不到了：{error}喵~"))?;
    let target = path
        .canonicalize()
        .map_err(|error| format!("这个克隆找不到了：{error}喵~"))?;
    if target == root || !target.starts_with(&root) {
        return Err("这个目录不在本地克隆目录里，人家不敢删喵！".to_string());
    }
    if !target.join(".git").exists() {
        return Err("这个目录不是 git 克隆，人家不敢删喵！".to_string());
    }
    std::fs::remove_dir_all(&target).map_err(|error| format!("删除没成功：{error}喵~"))
}

/// 把 owner/name、https 或 ssh 地址整理成 owner/name 喵~
pub fn parse_repo_ref(raw: &str) -> Result<(String, String), String> {
    let mut value = raw.trim().trim_end_matches('/');
    if let Some(stripped) = value.strip_suffix(".git") {
        value = stripped;
    }
    let value = github_path(value).unwrap_or(value);
    let (owner, name) = value
        .split_once('/')
        .ok_or_else(|| "要写成 owner/name 或者 GitHub 链接哦喵~".to_string())?;
    if name.contains('/') || !valid_repo_part(owner) || !valid_repo_part(name) {
        return Err("仓库名看起来不太对，检查一下 owner/name 嘛喵~".to_string());
    }
    Ok((format!("{owner}/{name}"), name.to_string()))
}

fn github_path(value: &str) -> Option<&str> {
    for prefix in [
        "git@github.com:",
        "ssh://git@github.com/",
        "https://github.com/",
        "http://github.com/",
    ] {
        if let Some(rest) = value.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    // 代理地址会把 github.com 放在中间，例如 https://proxy/https://github.com/a/b
    for needle in ["github.com/", "github.com:"] {
        if let Some(index) = value.find(needle) {
            let boundary =
                index == 0 || matches!(value[..index].chars().next_back(), Some('/') | Some('@'));
            if boundary {
                return Some(&value[index + needle.len()..]);
            }
        }
    }
    None
}

fn valid_repo_part(part: &str) -> bool {
    !part.is_empty()
        && part != "."
        && part != ".."
        && part
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

pub fn github_full_name(remote: &str) -> Option<String> {
    let mut value = remote.trim().trim_end_matches('/');
    if let Some(stripped) = value.strip_suffix(".git") {
        value = stripped;
    }
    let rest = github_path(value)?;
    let (owner, name) = rest.split_once('/')?;
    if name.contains('/') || !valid_repo_part(owner) || !valid_repo_part(name) {
        return None;
    }
    Some(format!("{owner}/{name}"))
}

fn parse_status(raw: &str) -> (String, String, u32, u32, Vec<StatusFile>) {
    let mut branch = "HEAD".to_string();
    let mut upstream = String::new();
    let mut ahead = 0;
    let mut behind = 0;
    let mut files = Vec::new();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            let (head, bracket) = rest.split_once(" [").unwrap_or((rest, ""));
            if let Some(rest) = head.strip_prefix("No commits yet on ") {
                branch = rest.to_string();
            } else if let Some((local, remote)) = head.split_once("...") {
                branch = local.to_string();
                upstream = remote.to_string();
            } else {
                branch = head.to_string();
            }
            ahead = bracket_count(bracket, "ahead");
            behind = bracket_count(bracket, "behind");
            continue;
        }
        let mut chars = line.chars();
        let index = chars.next().unwrap_or(' ');
        let worktree = chars.next().unwrap_or(' ');
        if index == ' ' && worktree == ' ' {
            continue;
        }
        let rest = line.chars().skip(3).collect::<String>();
        let path = rest
            .split_once(" -> ")
            .map(|(_, new)| new)
            .unwrap_or(&rest)
            .trim()
            .to_string();
        if path.is_empty() {
            continue;
        }
        files.push(StatusFile {
            path,
            index,
            worktree,
        });
    }
    (branch, upstream, ahead, behind, files)
}

fn bracket_count(bracket: &str, key: &str) -> u32 {
    let needle = format!("{key} ");
    bracket
        .split(',')
        .find_map(|part| part.trim().strip_prefix(&needle))
        .and_then(|value| value.trim_end_matches(']').trim().parse::<u32>().ok())
        .unwrap_or(0)
}

fn parse_commits(raw: &str) -> Vec<Commit> {
    raw.lines()
        .filter_map(|line| {
            let mut fields = line.split('\x1f');
            let short = fields.next()?.to_string();
            let author = fields.next()?.to_string();
            let when = fields.next()?.to_string();
            let summary = fields.next().unwrap_or("").to_string();
            Some(Commit {
                short,
                author,
                when,
                summary,
            })
        })
        .collect()
}

fn parse_tags(raw: &str) -> Vec<Tag> {
    raw.lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?.to_string();
            let when = fields.next().unwrap_or("").to_string();
            let subject = fields.next().unwrap_or("").to_string();
            Some(Tag {
                name,
                when,
                subject,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_header_and_files() {
        let raw = "## main...origin/main [ahead 2, behind 1]\n M src/main.rs\nA  src/cli.rs\n?? notes.md\nR  old.rs -> new.rs\n";
        let (branch, upstream, ahead, behind, files) = parse_status(raw);
        assert_eq!(branch, "main");
        assert_eq!(upstream, "origin/main");
        assert_eq!(ahead, 2);
        assert_eq!(behind, 1);
        assert_eq!(files.len(), 4);
        assert!(!files[0].untracked());
        assert!(files[2].untracked());
        assert_eq!(files[3].path, "new.rs");
    }

    #[test]
    fn parses_empty_repo_header() {
        let raw = "## No commits yet on main\n?? README.md\n";
        let (branch, upstream, ahead, behind, files) = parse_status(raw);
        assert_eq!(branch, "main");
        assert!(upstream.is_empty());
        assert_eq!(ahead, 0);
        assert_eq!(behind, 0);
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn rejects_non_repo_and_missing_paths() {
        let dir = std::env::temp_dir().join(format!("gxt-non-repo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let error = LocalRepo::open(&dir).unwrap_err();
        assert!(error.contains("不是 git 仓库"));
        let missing = dir.join("nope");
        assert!(LocalRepo::open(&missing).unwrap_err().contains("不存在"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn parses_commits_and_tags() {
        let commits = parse_commits("a1b2c3\x1fme\x1f2 hours ago\x1ffix: 喵\n");
        assert_eq!(commits[0].short, "a1b2c3");
        assert_eq!(commits[0].author, "me");
        assert_eq!(commits[0].summary, "fix: 喵");
        let tags = parse_tags("v0.1.2\t2 days ago\tRelease 0.1.2\n");
        assert_eq!(tags[0].name, "v0.1.2");
        assert_eq!(tags[0].subject, "Release 0.1.2");
    }

    #[test]
    fn parses_repo_refs_and_remotes() {
        for raw in [
            "octo/demo",
            "https://github.com/octo/demo",
            "https://github.com/octo/demo.git",
            "git@github.com:octo/demo.git",
            "ssh://git@github.com/octo/demo.git",
        ] {
            assert_eq!(
                parse_repo_ref(raw).unwrap(),
                ("octo/demo".to_string(), "demo".to_string())
            );
        }
        assert!(parse_repo_ref("octo").is_err());
        assert!(parse_repo_ref("../demo").is_err());
        assert!(parse_repo_ref("octo/demo/extra").is_err());
        assert_eq!(
            github_full_name("git@github.com:octo/demo.git").as_deref(),
            Some("octo/demo")
        );
        assert_eq!(
            github_full_name("https://v4.gh-proxy.org/https://github.com/octo/demo.git").as_deref(),
            Some("octo/demo")
        );
        assert_eq!(github_full_name("https://gitlab.com/octo/demo"), None);
    }

    #[test]
    fn checks_pulls_and_deletes_local_clone() {
        let base = std::env::temp_dir().join(format!("gxt-local-ops-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("clones");
        std::fs::create_dir_all(&root).unwrap();
        let remote = base.join("remote.git");
        let work = base.join("work");
        let target = root.join("target");
        let run = |dir: Option<&Path>, args: &[&str]| {
            let mut command = std::process::Command::new("git");
            if let Some(dir) = dir {
                command.arg("-C").arg(dir);
            }
            command
                .args(args)
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        };
        assert!(run(
            None,
            &["init", "--bare", "-q", remote.to_str().unwrap()]
        ));
        assert!(run(
            None,
            &["init", "-q", "-b", "main", work.to_str().unwrap()]
        ));
        assert!(run(
            Some(&work),
            &["config", "user.email", "test@example.com"]
        ));
        assert!(run(Some(&work), &["config", "user.name", "gxt"]));
        std::fs::write(work.join("README.md"), "hello\n").unwrap();
        assert!(run(Some(&work), &["add", "-A"]));
        assert!(run(Some(&work), &["commit", "-qm", "init"]));
        assert!(run(
            Some(&work),
            &["remote", "add", "origin", remote.to_str().unwrap()]
        ));
        assert!(run(Some(&work), &["push", "-q", "-u", "origin", "main"]));
        assert!(run(
            None,
            &[
                "clone",
                "-q",
                "-b",
                "main",
                remote.to_str().unwrap(),
                target.to_str().unwrap()
            ]
        ));

        let checked = check_clone(&target).unwrap();
        assert_eq!(checked.behind, 0);

        std::fs::write(work.join("README.md"), "hello\nmore\n").unwrap();
        assert!(run(Some(&work), &["add", "-A"]));
        assert!(run(Some(&work), &["commit", "-qm", "update"]));
        assert!(run(Some(&work), &["push", "-q", "origin", "main"]));

        let checked = check_clone(&target).unwrap();
        assert!(checked.behind >= 1);
        let pulled = pull_clone(&target).unwrap();
        assert_eq!(pulled.behind, 0);
        assert_eq!(pulled.dirty, 0);

        delete_clone(&root, &target).unwrap();
        assert!(!target.exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn delete_clone_refuses_paths_outside_root() {
        let base = std::env::temp_dir().join(format!("gxt-local-delete-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("clones");
        let outside = base.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        assert!(
            std::process::Command::new("git")
                .args(["init", "-q"])
                .arg(&outside)
                .status()
                .unwrap()
                .success()
        );
        assert!(delete_clone(&root, &outside).is_err());
        assert!(outside.exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn scans_git_directories_in_clone_root() {
        let root = std::env::temp_dir().join(format!("gxt-local-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("demo");
        std::fs::create_dir_all(&repo).unwrap();
        let init = std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(&repo)
            .status()
            .unwrap();
        assert!(init.success());
        let remote = std::process::Command::new("git")
            .args(["-C"])
            .arg(&repo)
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/octo/demo.git",
            ])
            .status()
            .unwrap();
        assert!(remote.success());
        let clones = scan_clones(&root).unwrap();
        assert_eq!(clones.len(), 1);
        assert_eq!(clones[0].name, "demo");
        assert_eq!(clones[0].full_name.as_deref(), Some("octo/demo"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
