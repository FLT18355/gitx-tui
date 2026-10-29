mod api;
mod app;
mod cache;
mod config;
mod format;
mod icons;
mod model;
mod theme;
mod ui;

use anyhow::Result;
use app::App;
use config::Paths;
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyEventKind,
};
use crossterm::execute;
use std::io;
use std::panic;
use std::time::Duration;

fn main() -> Result<()> {
    let mut theme_override = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-V" => {
                println!("gitx-tui {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--theme" => {
                theme_override = args.next();
            }
            value if value.starts_with("--theme=") => {
                theme_override = Some(value.trim_start_matches("--theme=").to_string());
            }
            other => {
                eprintln!("不认识的参数：{other}喵~");
                eprintln!("用 --help 看看怎么用喵~");
                std::process::exit(2);
            }
        }
    }

    let paths = Paths::discover();
    let mut terminal = ratatui::init();
    let _ = execute!(io::stdout(), EnableMouseCapture, EnableBracketedPaste);
    install_panic_hook();

    let mut app = App::new(paths, theme_override);
    let result = run(&mut terminal, &mut app);

    let _ = execute!(io::stdout(), DisableMouseCapture, DisableBracketedPaste);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        app.tick();
        app.drain_messages();
        terminal.draw(|frame| ui::draw(frame, app))?;
        if app.should_quit {
            break;
        }
        if event::poll(Duration::from_millis(100))? {
            loop {
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                            app.handle_key(key);
                        }
                    }
                    Event::Mouse(mouse) => app.handle_mouse(mouse),
                    Event::Paste(text) => app.handle_paste(&text),
                    Event::Resize(_, _) => {}
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn install_panic_hook() {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), DisableMouseCapture, DisableBracketedPaste);
        ratatui::restore();
        hook(info);
    }));
}

fn print_help() {
    println!(
        "gitx-tui {}\n\n\
终端里的 GitHub 工作台喵~用 gh 登录后，可以查看仓库、收藏、搜索、账号和仓库详情喵~\n\n\
用法：\n\
  gitx-tui [--theme <latte|frappe|macchiato|mocha>]\n\n\
按键：\n\
  1 / 2 / 3 / 4 / 5   切换仓库、收藏、搜索、账号、设置喵~\n\
  j / k               移动选择喵~\n\
  [ / ]               上一页 / 下一页喵~\n\
  g / G               第一页 / 最后一页喵~\n\
  Enter               打开仓库页喵~\n\
  e                   编辑自己的仓库喵~\n\
  o                   用浏览器打开主页喵~\n\
  y                   复制克隆地址喵~\n\
  r                   刷新当前页喵~\n\
  /                   聚焦搜索喵~\n\
  Ctrl+S              保存设置喵~\n\
  ?                   操作说明喵~\n\
  q                   退出喵~\n\n\
配置文件：~/.config/gitx-tui/config.toml\n\
缓存目录：~/.cache/gitx-tui",
        env!("CARGO_PKG_VERSION")
    );
}
