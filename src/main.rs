mod api;
mod app;
mod cache;
mod cli;
mod commands;
mod config;
mod format;
mod icons;
mod local;
mod local_app;
mod local_git;
mod local_ops;
mod local_ui;
mod model;
mod theme;
mod ui;

use anyhow::Result;
use app::App;
use cli::Command;
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
    let command = match cli::parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let command = match command {
        Command::Help => {
            cli::print_help();
            return Ok(());
        }
        Command::Version => {
            cli::print_version();
            return Ok(());
        }
        Command::Status { path } => return finish(commands::status(&path)),
        Command::Log { path, limit } => return finish(commands::log(&path, limit)),
        Command::Tags { path } => return finish(commands::tags(&path)),
        Command::Push { path } => return finish(commands::push(&path)),
        Command::Config { show, path_only } => {
            return finish(commands::config(show, path_only));
        }
        Command::Cache { action } => return finish(commands::cache(action)),
        Command::Doctor => return finish(commands::doctor()),
        command @ (Command::Github { .. } | Command::Settings { .. } | Command::Local { .. }) => {
            command
        }
    };

    let paths = Paths::discover();
    let mut terminal = ratatui::init();
    let _ = execute!(io::stdout(), EnableMouseCapture, EnableBracketedPaste);
    install_panic_hook();

    let result = match command {
        Command::Github { theme } => {
            let mut app = App::new(paths, theme);
            run(&mut terminal, &mut app)
        }
        Command::Settings { theme } => {
            let mut app = App::settings(paths, theme);
            run(&mut terminal, &mut app)
        }
        Command::Local { path, theme } => local_app::run(&mut terminal, &path, theme),
        _ => unreachable!(),
    };

    let _ = execute!(io::stdout(), DisableMouseCapture, DisableBracketedPaste);
    ratatui::restore();
    result
}

fn finish(result: Result<(), String>) -> Result<()> {
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
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
