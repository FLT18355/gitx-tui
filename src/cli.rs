use crate::theme::{FLAVORS, is_valid_name};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheAction {
    Stats,
    Path,
    Clear,
    Prune,
}

pub enum ConfigAction {
    Summary,
    Show,
    Path,
    Get(String),
    Set(String, String),
}

pub enum Command {
    Github {
        theme: Option<String>,
    },
    Md {
        path: PathBuf,
        theme: Option<String>,
    },
    Settings {
        theme: Option<String>,
    },
    Status {
        path: PathBuf,
    },
    Log {
        path: PathBuf,
        limit: usize,
    },
    Tags {
        path: PathBuf,
    },
    Push {
        path: PathBuf,
    },
    Config {
        action: ConfigAction,
    },
    Commits {
        repo: String,
        limit: usize,
    },
    Branches {
        repo: String,
    },
    Switch {
        branch: String,
        path: PathBuf,
    },
    Cache {
        action: CacheAction,
    },
    Doctor,
    Help,
    Version,
}

pub fn parse<I>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    let mut theme = None;
    let mut limit = 20;
    let mut positional: Vec<String> = Vec::new();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" | "help" => return Ok(Command::Help),
            "-V" | "--version" | "version" => return Ok(Command::Version),
            "--theme" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--theme 后面要写主题名喵~".to_string())?;
                theme = Some(validate_theme(value)?);
            }
            _ if arg.starts_with("--theme=") => {
                let value = arg.trim_start_matches("--theme=").to_string();
                theme = Some(validate_theme(value)?);
            }
            "-n" | "--limit" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "-n 后面要写条数喵~".to_string())?;
                limit = value
                    .parse::<usize>()
                    .ok()
                    .filter(|value| *value > 0)
                    .ok_or_else(|| format!("条数要写正整数喵~：{value}"))?;
            }
            _ if arg.starts_with('-') && arg != "-" => {
                return Err(format!("不认识的参数：{arg}喵~敲 gxt --help 看看用法吧喵~"));
            }
            _ => positional.push(arg),
        }
    }
    let path = |value: &str| PathBuf::from(value);
    match positional.as_slice() {
        [] => Ok(Command::Github { theme }),
        [one] if one == "settings" => Ok(Command::Settings { theme }),
        [one] if one == "md" => Ok(Command::Md {
            path: path("."),
            theme,
        }),
        [one, two] if one == "md" => Ok(Command::Md {
            path: path(two),
            theme,
        }),
        [one, ..] if one == "md" => Err("md 后面最多带一个目录或文件喵~".to_string()),
        [one, ..] if one == "settings" => Err("settings 后面不用带参数喵~".to_string()),
        [one] if one == "status" => Ok(Command::Status { path: path(".") }),
        [one, two] if one == "status" => Ok(Command::Status { path: path(two) }),
        [one] if one == "log" => Ok(Command::Log {
            path: path("."),
            limit,
        }),
        [one, two] if one == "log" => Ok(Command::Log {
            path: path(two),
            limit,
        }),
        [one] if one == "tags" => Ok(Command::Tags { path: path(".") }),
        [one, two] if one == "tags" => Ok(Command::Tags { path: path(two) }),
        [one] if one == "push" => Ok(Command::Push { path: path(".") }),
        [one, two] if one == "push" => Ok(Command::Push { path: path(two) }),
        [one] if one == "config" => Ok(Command::Config {
            action: ConfigAction::Summary,
        }),
        [one, two] if one == "config" => match two.as_str() {
            "show" => Ok(Command::Config {
                action: ConfigAction::Show,
            }),
            "path" => Ok(Command::Config {
                action: ConfigAction::Path,
            }),
            _ => Err(format!(
                "config 支持 show / path / get <键> / set <键> <值> 喵~：{two}"
            )),
        },
        [one, two, three] if one == "config" && two == "get" => Ok(Command::Config {
            action: ConfigAction::Get(three.to_string()),
        }),
        [one, two, three, four] if one == "config" && two == "set" => Ok(Command::Config {
            action: ConfigAction::Set(three.to_string(), four.to_string()),
        }),
        [one, two] if one == "commits" => Ok(Command::Commits {
            repo: two.to_string(),
            limit,
        }),
        [one, two, three] if one == "commits" && two == "-n" => Ok(Command::Commits {
            repo: three.to_string(),
            limit,
        }),
        [one, two] if one == "branches" => Ok(Command::Branches {
            repo: two.to_string(),
        }),
        [one, two] if one == "switch" => Ok(Command::Switch {
            branch: two.to_string(),
            path: path("."),
        }),
        [one, two, three] if one == "switch" => Ok(Command::Switch {
            branch: two.to_string(),
            path: path(three),
        }),
        [one] if one == "cache" => Ok(Command::Cache {
            action: CacheAction::Stats,
        }),
        [one, two] if one == "cache" => match two.as_str() {
            "clear" => Ok(Command::Cache {
                action: CacheAction::Clear,
            }),
            "prune" => Ok(Command::Cache {
                action: CacheAction::Prune,
            }),
            "path" => Ok(Command::Cache {
                action: CacheAction::Path,
            }),
            "stats" => Ok(Command::Cache {
                action: CacheAction::Stats,
            }),
            _ => Err(format!(
                "cache 只支持 stats / path / clear / prune 喵~：{two}"
            )),
        },
        [one] if one == "doctor" => Ok(Command::Doctor),
        [one] if matches!(one.as_str(), "commits" | "branches" | "switch") => Err(format!(
            "{one} 后面还要带仓库或分支参数喵~敲 gxt --help 看看用法吧喵~"
        )),
        _ => Err("不认识这个参数喵~敲 gxt --help 看看用法吧喵~".to_string()),
    }
}

fn validate_theme(value: String) -> Result<String, String> {
    let theme = value.trim().to_ascii_lowercase();
    if FLAVORS.contains(&theme.as_str()) || is_valid_name(&theme) {
        Ok(theme)
    } else {
        Err(format!(
            "主题名只能用字母、数字、- 和 _ 喵~内置主题：{}，自定义主题放到 ~/.config/gitx-tui/themes/<名字>.toml 喵~",
            FLAVORS.join(" / ")
        ))
    }
}

pub fn print_help() {
    println!(
        "gxt {}\n\
终端 GitHub 工作台喵~\n\n\
用法：\n\
  gxt [--theme <主题>]             打开 GitHub 工作台喵~\n\
  gxt settings [--theme <主题>]    打开全局设置 TUI喵~\n\
  gxt md [目录或文件]              打开 Markdown 预览器喵~\n\
  gxt status [路径]                打印本地仓库状态喵~\n\
  gxt log [路径] [-n <条数>]       打印最近提交喵~\n\
  gxt tags [路径]                  打印标签喵~\n\
  gxt push [路径]                  推送当前分支喵~\n\
  gxt config [show|path|get <键>|set <键> <值>]  查看或修改配置喵~\n\
  gxt commits <仓库> [-n <条数>]    打印远端仓库提交喵~\n\
  gxt branches <仓库>              打印远端仓库分支喵~\n\
  gxt switch <分支> [路径]         切换本地分支喵~\n\
  gxt cache [stats|path|clear|prune]  查看、清理或清旧缓存喵~\n\
  gxt doctor                       环境自检喵~\n\n\
选项：\n\
  -n, --limit <条数>                log 显示条数，默认 20 喵~\n\
  --theme <主题>                   latte / frappe / macchiato / mocha / violet 或自定义主题名喵~\n\
  -h, --help                       显示本帮助喵~\n\
  -V, --version                    显示版本喵~\n\n\
配置文件：~/.config/gitx-tui/config.toml\n\
缓存目录：~/.cache/gitx-tui\n\
键位说明：https://github.com/FLT18355/gitx-tui/blob/main/docs/KEYBINDINGS.md",
        env!("CARGO_PKG_VERSION")
    );
}

pub fn print_version() {
    println!("gxt {}", env!("CARGO_PKG_VERSION"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_modes_and_theme() {
        assert!(matches!(
            parse(args(&[])).unwrap(),
            Command::Github { theme: None }
        ));
        assert!(matches!(
            parse(args(&["settings", "--theme", "latte"])).unwrap(),
            Command::Settings {
                theme: Some(theme)
            } if theme == "latte"
        ));
        assert!(parse(args(&["/tmp/demo", "--theme=mocha"])).is_err());
        assert!(matches!(parse(args(&["--help"])).unwrap(), Command::Help));
        assert!(matches!(
            parse(args(&["md"])).unwrap(),
            Command::Md { path, theme: None } if path == std::path::Path::new(".")
        ));
        assert!(matches!(
            parse(args(&["md", "docs", "--theme", "latte"])).unwrap(),
            Command::Md { path, theme: Some(theme) }
                if path == std::path::Path::new("docs") && theme == "latte"
        ));
    }

    #[test]
    fn parses_utility_commands() {
        assert!(matches!(
            parse(args(&["status", "/tmp/demo"])).unwrap(),
            Command::Status { path } if path == std::path::Path::new("/tmp/demo")
        ));
        assert!(matches!(
            parse(args(&["log", "-n", "50"])).unwrap(),
            Command::Log { path, limit: 50 } if path == std::path::Path::new(".")
        ));
        assert!(matches!(
            parse(args(&["cache", "clear"])).unwrap(),
            Command::Cache {
                action: CacheAction::Clear
            }
        ));
        assert!(matches!(
            parse(args(&["cache", "prune"])).unwrap(),
            Command::Cache {
                action: CacheAction::Prune
            }
        ));
        assert!(matches!(
            parse(args(&["config", "show"])).unwrap(),
            Command::Config {
                action: ConfigAction::Show
            }
        ));
        assert!(matches!(
            parse(args(&["config", "path"])).unwrap(),
            Command::Config {
                action: ConfigAction::Path
            }
        ));
        assert!(matches!(
            parse(args(&["config", "get", "theme"])).unwrap(),
            Command::Config {
                action: ConfigAction::Get(key)
            } if key == "theme"
        ));
        assert!(matches!(
            parse(args(&["config", "set", "commit_limit", "80"])).unwrap(),
            Command::Config {
                action: ConfigAction::Set(key, value)
            } if key == "commit_limit" && value == "80"
        ));
        assert!(matches!(
            parse(args(&["commits", "ratatui/ratatui", "-n", "5"])).unwrap(),
            Command::Commits { repo, limit: 5 } if repo == "ratatui/ratatui"
        ));
        assert!(matches!(
            parse(args(&["branches", "ratatui/ratatui"])).unwrap(),
            Command::Branches { repo } if repo == "ratatui/ratatui"
        ));
        assert!(matches!(
            parse(args(&["switch", "main", "/tmp/demo"])).unwrap(),
            Command::Switch { branch, path }
                if branch == "main" && path == std::path::Path::new("/tmp/demo")
        ));
    }

    #[test]
    fn rejects_bad_arguments() {
        assert!(parse(args(&["--theme", "../bad"])).is_err());
        assert!(parse(args(&["settings", "extra"])).is_err());
        assert!(parse(args(&["a", "b"])).is_err());
        assert!(parse(args(&["--wat"])).is_err());
        assert!(parse(args(&["log", "-n", "0"])).is_err());
    }
}
