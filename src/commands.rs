use crate::api::ApiClient;
use crate::cache::Cache;
use crate::cli::{CacheAction, ConfigAction};
use crate::config::{Config, Paths};
use crate::format;
use crate::local::LocalRepo;
use crate::local_ops;
use crate::theme::Theme;
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

pub fn config(action: ConfigAction) -> Result<(), String> {
    let paths = Paths::discover();
    let mut config = Config::load(&paths);
    match action {
        ConfigAction::Show => {
            let raw = std::fs::read_to_string(paths.config_file())
                .map_err(|err| format!("配置文件读不出来喵~：{err}"))?;
            print!("{raw}");
            return Ok(());
        }
        ConfigAction::Path => {
            println!("{}", format::display_path(&paths.config_file()));
            return Ok(());
        }
        ConfigAction::Get(key) => {
            println!("{}", config_value(&config, &key)?);
            return Ok(());
        }
        ConfigAction::Set(key, value) => {
            set_config_value(&mut config, &paths, &key, &value)?;
            config
                .save(&paths)
                .map_err(|err| format!("配置没存上喵~：{err}"))?;
            println!("{key} = {}", config_value(&config, &key)?);
            return Ok(());
        }
        ConfigAction::Summary => {}
    }
    println!("配置文件：{}", format::display_path(&paths.config_file()));
    println!("主题：{}", config.theme);
    println!("代理：{}", config.proxy);
    println!("下载目录：{}", config.download_dir);
    println!("提交条数：{}", config.commit_limit);
    Ok(())
}

fn config_value(config: &Config, key: &str) -> Result<String, String> {
    match key {
        "theme" => Ok(config.theme.clone()),
        "proxy" => Ok(config.proxy.clone()),
        "repos_ttl_secs" => Ok(config.repos_ttl_secs.to_string()),
        "detail_ttl_secs" => Ok(config.detail_ttl_secs.to_string()),
        "search_ttl_secs" => Ok(config.search_ttl_secs.to_string()),
        "show_file_tree" => Ok(config.show_file_tree.to_string()),
        "show_readme" => Ok(config.show_readme.to_string()),
        "download_dir" => Ok(config.download_dir.clone()),
        "commit_limit" => Ok(config.commit_limit.to_string()),
        _ => Err(format!(
            "不认识的配置键：{key}喵~支持 theme / proxy / repos_ttl_secs / detail_ttl_secs / search_ttl_secs / show_file_tree / show_readme / download_dir / commit_limit"
        )),
    }
}

fn set_config_value(
    config: &mut Config,
    paths: &Paths,
    key: &str,
    value: &str,
) -> Result<(), String> {
    let boolean = || match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "on" | "yes" => Ok(true),
        "false" | "0" | "off" | "no" => Ok(false),
        _ => Err(format!("这个键要写 true 或 false 喵~：{value}")),
    };
    let number = || {
        value
            .trim()
            .parse::<u64>()
            .map_err(|_| format!("这个键要写正整数喵~：{value}"))
    };
    match key {
        "theme" => {
            let theme = value.trim().to_ascii_lowercase();
            if !Theme::exists(paths, &theme) {
                return Err(format!(
                    "主题 {theme} 还没装喵~把主题文件放到 ~/.config/gitx-tui/themes/{theme}.toml 再试喵~"
                ));
            }
            config.theme = theme;
        }
        "proxy" => config.proxy = value.trim().to_string(),
        "repos_ttl_secs" => config.repos_ttl_secs = number()?,
        "detail_ttl_secs" => config.detail_ttl_secs = number()?,
        "search_ttl_secs" => config.search_ttl_secs = number()?,
        "show_file_tree" => config.show_file_tree = boolean()?,
        "show_readme" => config.show_readme = boolean()?,
        "download_dir" => {
            if value.trim().is_empty() {
                return Err("下载目录不能空着喵~".to_string());
            }
            config.download_dir = value.trim().to_string();
        }
        "commit_limit" => {
            config.commit_limit = value
                .trim()
                .parse::<usize>()
                .ok()
                .filter(|limit| (10..=100).contains(limit))
                .ok_or_else(|| "commit_limit 要写 10 到 100 喵~".to_string())?;
        }
        _ => return config_value(config, key).map(|_| ()),
    }
    Ok(())
}

pub fn remote_commits(repo: &str, limit: usize) -> Result<(), String> {
    let paths = Paths::discover();
    let config = Config::load(&paths);
    let client = ApiClient::new(config.proxy.clone());
    let detail = client
        .repository(repo, true)
        .or_else(|_| client.repository(repo, false))?;
    let branch = detail.default_branch.clone();
    let commits = client.commits(repo, &branch, limit, detail.private)?;
    println!("{repo} · {branch}");
    if commits.is_empty() {
        println!("这个分支还没有提交喵~");
        return Ok(());
    }
    for commit in commits {
        println!(
            "{}  {}  ({}, {})",
            commit.short_sha(),
            commit.summary(),
            commit.author_name(),
            format::relative_time(&commit.commit.author.date)
        );
    }
    Ok(())
}

pub fn remote_branches(repo: &str) -> Result<(), String> {
    let paths = Paths::discover();
    let config = Config::load(&paths);
    let client = ApiClient::new(config.proxy.clone());
    let branches = client
        .branches(repo, true)
        .or_else(|_| client.branches(repo, false))?;
    if branches.is_empty() {
        println!("这个仓库还没有分支喵~");
        return Ok(());
    }
    for branch in branches {
        let sha = branch.commit.sha.get(..7).unwrap_or(&branch.commit.sha);
        println!("{}  {}", branch.name, sha);
    }
    Ok(())
}

pub fn switch_branch(path: &Path, branch: &str) -> Result<(), String> {
    let repo = LocalRepo::open(path)?;
    local_ops::checkout(&repo.root, branch)?;
    println!("切到 {branch} 分支啦喵~");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_set_validates_values() {
        let mut config = Config::default();
        let root = std::env::temp_dir().join(format!("gitx-tui-cmd-{}", std::process::id()));
        let paths = Paths::for_test(root.join("config"), root.join("cache"));
        set_config_value(&mut config, &paths, "commit_limit", "80").unwrap();
        assert_eq!(config.commit_limit, 80);
        assert!(set_config_value(&mut config, &paths, "commit_limit", "5").is_err());
        set_config_value(&mut config, &paths, "show_readme", "on").unwrap();
        assert!(config.show_readme);
        assert!(set_config_value(&mut config, &paths, "show_readme", "maybe").is_err());
        assert!(set_config_value(&mut config, &paths, "theme", "neon").is_err());
        assert!(set_config_value(&mut config, &paths, "nope", "1").is_err());
    }
}
