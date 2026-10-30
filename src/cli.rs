use crate::theme::FLAVORS;
use std::path::PathBuf;

pub enum Command {
    Github {
        theme: Option<String>,
    },
    Settings {
        theme: Option<String>,
    },
    Local {
        path: PathBuf,
        theme: Option<String>,
    },
    Help,
    Version,
}

pub fn parse<I>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    let mut theme = None;
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
            _ if arg.starts_with('-') && arg != "-" => {
                return Err(format!("不认识的参数：{arg}喵~敲 gxt --help 看看用法吧喵~"));
            }
            _ => positional.push(arg),
        }
    }
    match positional.as_slice() {
        [] => Ok(Command::Github { theme }),
        [one] if one == "settings" => Ok(Command::Settings { theme }),
        [one] => Ok(Command::Local {
            path: PathBuf::from(one),
            theme,
        }),
        [one, ..] if one == "settings" => Err("settings 后面不用带参数喵~".to_string()),
        _ => Err("参数太多啦喵~敲 gxt --help 看看用法吧喵~".to_string()),
    }
}

fn validate_theme(value: String) -> Result<String, String> {
    let theme = value.trim().to_ascii_lowercase();
    if FLAVORS.contains(&theme.as_str()) {
        Ok(theme)
    } else {
        Err(format!(
            "不认识的喵主题：{value}喵~只能选 {} 喵~",
            FLAVORS.join(" / ")
        ))
    }
}

pub fn print_help() {
    println!(
        "gxt {}\n\n\
终端里的 GitHub 工作台喵~\n\n\
用法：\n\
  gxt                         打开 GitHub 工作台喵~\n\
  gxt <路径>                  打开本地仓库的 lazygit 风格界面喵~\n\
  gxt settings                打开全局设置 TUI 喵~\n\
  gxt --theme <主题>          指定 latte / frappe / macchiato / mocha 喵~\n\
  gxt --help                  看这段帮助喵~\n\
  gxt --version               看版本喵~\n\n\
GitHub 工作台按键：\n\
  1 / 2 / 3 / 4      仓库 / 收藏 / 搜索 / 账号喵~\n\
  j / k              移动选择喵~\n\
  [ / ]              上一页 / 下一页喵~\n\
  Enter              打开仓库页喵~\n\
  a                  看当前仓库的 Actions 工作流喵~\n\
  R / x              在 Actions 页手动运行工作流喵~\n\
  c                  管理本地缓存喵~\n\
  e                  编辑自己的仓库喵~\n\
  o                  用浏览器打开主页喵~\n\
  y                  复制克隆地址喵~\n\
  r                  刷新当前页喵~\n\
  /                  聚焦搜索喵~\n\
  Ctrl+S             保存设置喵~\n\
  ?                  操作说明喵~\n\
  q                  退出喵~\n\n\
设置 TUI 按键：\n\
  j / k              选设置项喵~\n\
  h / l              调整当前项喵~\n\
  Enter              执行当前项喵~\n\
  Ctrl+S             保存配置喵~\n\
  Esc                退出喵~\n\n\
本地仓库按键：\n\
  j / k              挑文件或提交喵~\n\
  Tab                切换改动 / 提交 / 分支面板喵~\n\
  s / u              暂存 / 取消暂存选中文件喵~\n\
  c                  写提交信息并提交喵~\n\
  r                  重新读取仓库喵~\n\
  q / Esc            退出喵~\n\n\
配置文件：~/.config/gitx-tui/config.toml\n\
缓存目录：~/.cache/gitx-tui",
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
        assert!(matches!(
            parse(args(&["/tmp/demo", "--theme=mocha"])).unwrap(),
            Command::Local { path, theme: Some(theme) }
                if path == std::path::Path::new("/tmp/demo") && theme == "mocha"
        ));
        assert!(matches!(parse(args(&["--help"])).unwrap(), Command::Help));
    }

    #[test]
    fn rejects_bad_arguments() {
        assert!(parse(args(&["--theme", "neon"])).is_err());
        assert!(parse(args(&["settings", "extra"])).is_err());
        assert!(parse(args(&["a", "b"])).is_err());
        assert!(parse(args(&["--wat"])).is_err());
    }
}
