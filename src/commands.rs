use crate::cache::Cache;
use crate::cli::CacheAction;
use crate::config::{Config, Paths};
use crate::format;
use crate::local::LocalRepo;
use crate::local_ops;
use std::path::Path;
use std::process::Command;

pub fn status(path: &Path) -> Result<(), String> {
    let repo = LocalRepo::open(path)?;
    println!("{}  [{}]", repo.display_root(), repo.branch);
    if !repo.upstream.is_empty() {
        let mut tracking = format!("upstream {}", repo.upstream);
        if repo.ahead > 0 {
            tracking.push_str(&format!("  ahead {}", repo.ahead));
        }
        if repo.behind > 0 {
            tracking.push_str(&format!("  behind {}", repo.behind));
        }
        println!("{tracking}");
    }
    if repo.files.is_empty() {
        println!("工作区干干净净喵~");
        return Ok(());
    }
    for file in &repo.files {
        let mark = if file.untracked() {
            "??".to_string()
        } else {
            format!("{}{}", file.index, file.worktree)
                .trim()
                .to_string()
        };
        println!("{:<2}  {:<6} {}", mark, file.status_label(), file.path);
    }
    Ok(())
}

pub fn log(path: &Path, limit: usize) -> Result<(), String> {
    let repo = LocalRepo::open(path)?;
    if repo.commits.is_empty() {
        println!("还没有提交喵~");
        return Ok(());
    }
    for commit in repo.commits.iter().take(limit) {
        println!(
            "{}  {}  ({}, {})",
            commit.short, commit.summary, commit.author, commit.when
        );
    }
    Ok(())
}

pub fn tags(path: &Path) -> Result<(), String> {
    let repo = LocalRepo::open(path)?;
    if repo.tags.is_empty() {
        println!("还没有标签喵~");
        return Ok(());
    }
    for tag in &repo.tags {
        if tag.subject.is_empty() {
            println!("{}  {}", tag.name, tag.when);
        } else {
            println!("{}  {}  {}", tag.name, tag.when, tag.subject);
        }
    }
    Ok(())
}

pub fn push(path: &Path) -> Result<(), String> {
    let repo = LocalRepo::open(path)?;
    let message = local_ops::push(&repo.root, &repo.branch)?;
    println!("{message}");
    Ok(())
}

pub fn config(show: bool, path_only: bool) -> Result<(), String> {
    let paths = Paths::discover();
    let config = Config::load(&paths);
    if show {
        let raw = std::fs::read_to_string(paths.config_file())
            .map_err(|err| format!("配置文件读不出来喵~：{err}"))?;
        print!("{raw}");
        return Ok(());
    }
    if path_only {
        println!("{}", format::display_path(&paths.config_file()));
        return Ok(());
    }
    println!("配置文件：{}", format::display_path(&paths.config_file()));
    println!("主题：{}", config.theme);
    println!("代理：{}", config.proxy);
    println!("下载目录：{}", config.download_dir);
    Ok(())
}

pub fn cache(action: CacheAction) -> Result<(), String> {
    let paths = Paths::discover();
    let cache = Cache::new(paths.cache_dir().to_path_buf());
    match action {
        CacheAction::Path => {
            println!("{}", format::display_path(paths.cache_dir()));
        }
        CacheAction::Stats => {
            let stats = cache.stats();
            println!("缓存目录：{}", format::display_path(paths.cache_dir()));
            println!(
                "{} 个文件 · {}",
                stats.files,
                format::format_size(stats.bytes)
            );
        }
        CacheAction::Clear => {
            cache
                .clear()
                .map_err(|err| format!("缓存没清掉喵~：{err}"))?;
            println!("缓存清掉啦喵~");
        }
    }
    Ok(())
}

pub fn doctor() -> Result<(), String> {
    let paths = Paths::discover();
    let config = Config::load(&paths);
    let cache = Cache::new(paths.cache_dir().to_path_buf());
    let stats = cache.stats();
    let git = Command::new("git")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|| "没有找到 git 喵~".to_string());
    let token = env_token()
        .or_else(|| {
            Command::new("gh")
                .args(["auth", "token", "--hostname", "github.com"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
                .filter(|token| !token.is_empty())
        })
        .is_some();
    println!("gxt {}", env!("CARGO_PKG_VERSION"));
    println!("git：{git}");
    println!(
        "GitHub 登录：{}",
        if token {
            "已登录喵~"
        } else {
            "未登录喵~"
        }
    );
    println!("主题：{}", config.theme);
    println!("代理：{}", config.proxy);
    println!("配置文件：{}", format::display_path(&paths.config_file()));
    println!(
        "缓存目录：{}（{} 个文件 · {}）",
        format::display_path(paths.cache_dir()),
        stats.files,
        format::format_size(stats.bytes)
    );
    Ok(())
}

fn env_token() -> Option<String> {
    ["GITHUB_TOKEN", "GH_TOKEN"]
        .iter()
        .find_map(|key| std::env::var(key).ok())
        .filter(|value| !value.trim().is_empty())
}
