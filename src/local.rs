use crate::local_git::git;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct StatusFile {
    pub path: String,
    pub index: char,
    pub worktree: char,
}

impl StatusFile {
    pub fn staged(&self) -> bool {
        self.index != ' ' && self.index != '?'
    }

    pub fn untracked(&self) -> bool {
        self.index == '?' && self.worktree == '?'
    }

    pub fn unstaged(&self) -> bool {
        self.worktree != ' ' || self.untracked()
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
    pub hash: String,
    pub short: String,
    pub author: String,
    pub when: String,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub name: String,
    pub current: bool,
    pub upstream: String,
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
    pub branches: Vec<Branch>,
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
            &[
                "log",
                "-n",
                "100",
                "--pretty=format:%H%x1f%h%x1f%an%x1f%ar%x1f%s",
            ],
        )
        .map(|raw| parse_commits(&raw))
        .unwrap_or_default();
        let branches = git(
            &root,
            &[
                "branch",
                "--format=%(refname:short)\t%(HEAD)\t%(upstream:short)",
            ],
        )
        .map(|raw| parse_branches(&raw))
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
            branches,
            tags,
        })
    }

    pub fn staged_count(&self) -> usize {
        self.files.iter().filter(|file| file.staged()).count()
    }

    pub fn unstaged_count(&self) -> usize {
        self.files.iter().filter(|file| file.unstaged()).count()
    }

    pub fn display_root(&self) -> String {
        crate::format::display_path(&self.root)
    }
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
            let hash = fields.next()?.to_string();
            let short = fields.next()?.to_string();
            let author = fields.next()?.to_string();
            let when = fields.next()?.to_string();
            let summary = fields.next().unwrap_or("").to_string();
            Some(Commit {
                hash,
                short,
                author,
                when,
                summary,
            })
        })
        .collect()
}

fn parse_branches(raw: &str) -> Vec<Branch> {
    raw.lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?.to_string();
            let current = fields.next().unwrap_or("") == "*";
            let upstream = fields.next().unwrap_or("").to_string();
            Some(Branch {
                name,
                current,
                upstream,
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
        assert!(files[0].unstaged());
        assert!(files[1].staged());
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
    fn parses_commits_branches_and_tags() {
        let commits = parse_commits("abc123\x1fa1b2c3\x1fme\x1f2 hours ago\x1ffix: 喵\n");
        assert_eq!(commits[0].hash, "abc123");
        assert_eq!(commits[0].short, "a1b2c3");
        assert_eq!(commits[0].summary, "fix: 喵");
        let branches = parse_branches("main\t*\torigin/main\nfeature\t\t\n");
        assert!(branches[0].current);
        assert_eq!(branches[0].upstream, "origin/main");
        assert!(!branches[1].current);
        let tags = parse_tags("v0.1.2\t2 days ago\tRelease 0.1.2\n");
        assert_eq!(tags[0].name, "v0.1.2");
        assert_eq!(tags[0].subject, "Release 0.1.2");
    }
}
