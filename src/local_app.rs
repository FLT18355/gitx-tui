use crate::app::TextInput;
use crate::config::{Config, Paths};
use crate::local::{self, LocalRepo};
use crate::local_ui;
use crate::theme::{FLAVORS, Theme};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalPane {
    Status,
    Commits,
    Branches,
}

pub struct LocalApp {
    pub theme: Theme,
    pub repo: LocalRepo,
    pub pane: LocalPane,
    pub file_selected: usize,
    pub commit_selected: usize,
    pub branch_selected: usize,
    pub message: String,
    pub message_error: bool,
    pub commit_input: Option<TextInput>,
    pub should_quit: bool,
}

impl LocalApp {
    pub fn new(path: &Path, theme_override: Option<String>) -> Result<Self, String> {
        let paths = Paths::discover();
        let config = Config::load(&paths);
        let theme_name = theme_override
            .filter(|value| FLAVORS.contains(&value.as_str()))
            .unwrap_or_else(|| config.theme.clone());
        let theme = Theme::load(&paths, &theme_name);
        let repo = LocalRepo::open(path)?;
        let message = if repo.files.is_empty() {
            "工作区干干净净，没有改动喵~".to_string()
        } else {
            format!(
                "读到 {} 个改动、{} 个提交喵~",
                repo.files.len(),
                repo.commits.len()
            )
        };
        Ok(Self {
            theme,
            repo,
            pane: LocalPane::Status,
            file_selected: 0,
            commit_selected: 0,
            branch_selected: 0,
            message,
            message_error: false,
            commit_input: None,
            should_quit: false,
        })
    }

    fn refresh(&mut self) {
        match LocalRepo::open(&self.repo.root) {
            Ok(repo) => {
                self.repo = repo;
                self.clamp_selection();
                self.message = "仓库重新读好啦喵~".to_string();
                self.message_error = false;
            }
            Err(error) => {
                self.message = error;
                self.message_error = true;
            }
        }
    }

    fn clamp_selection(&mut self) {
        self.file_selected = self
            .file_selected
            .min(self.repo.files.len().saturating_sub(1));
        self.commit_selected = self
            .commit_selected
            .min(self.repo.commits.len().saturating_sub(1));
        self.branch_selected = self
            .branch_selected
            .min(self.repo.branches.len().saturating_sub(1));
    }

    fn selected_file(&self) -> Option<&local::StatusFile> {
        self.repo.files.get(self.file_selected)
    }

    fn move_selection(&mut self, delta: i32) {
        match self.pane {
            LocalPane::Status => {
                if self.repo.files.is_empty() {
                    return;
                }
                self.file_selected = (self.file_selected as i32 + delta)
                    .clamp(0, self.repo.files.len() as i32 - 1)
                    as usize;
            }
            LocalPane::Commits => {
                if self.repo.commits.is_empty() {
                    return;
                }
                self.commit_selected = (self.commit_selected as i32 + delta)
                    .clamp(0, self.repo.commits.len() as i32 - 1)
                    as usize;
            }
            LocalPane::Branches => {
                if self.repo.branches.is_empty() {
                    return;
                }
                self.branch_selected = (self.branch_selected as i32 + delta)
                    .clamp(0, self.repo.branches.len() as i32 - 1)
                    as usize;
            }
        }
    }

    fn set_selection_edge(&mut self, end: bool) {
        match self.pane {
            LocalPane::Status => {
                self.file_selected = if end {
                    self.repo.files.len().saturating_sub(1)
                } else {
                    0
                };
            }
            LocalPane::Commits => {
                self.commit_selected = if end {
                    self.repo.commits.len().saturating_sub(1)
                } else {
                    0
                };
            }
            LocalPane::Branches => {
                self.branch_selected = if end {
                    self.repo.branches.len().saturating_sub(1)
                } else {
                    0
                };
            }
        }
    }

    fn cycle_pane(&mut self) {
        self.pane = match self.pane {
            LocalPane::Status => LocalPane::Commits,
            LocalPane::Commits => LocalPane::Branches,
            LocalPane::Branches => LocalPane::Status,
        };
    }

    fn stage_selected(&mut self) {
        let Some(file) = self.selected_file().cloned() else {
            self.message = "没有选中的文件喵~".to_string();
            self.message_error = true;
            return;
        };
        if !file.unstaged() {
            self.message = "这个文件已经暂存过啦喵~".to_string();
            self.message_error = false;
            return;
        }
        match local::stage(&self.repo.root, &file.path) {
            Ok(()) => {
                self.refresh();
                self.message = format!("{} 暂存好啦喵~", file.path);
                self.message_error = false;
            }
            Err(error) => {
                self.message = error;
                self.message_error = true;
            }
        }
    }

    fn unstage_selected(&mut self) {
        let Some(file) = self.selected_file().cloned() else {
            self.message = "没有选中的文件喵~".to_string();
            self.message_error = true;
            return;
        };
        if !file.staged() {
            self.message = "这个文件还没暂存呢喵~".to_string();
            self.message_error = false;
            return;
        }
        match local::unstage(&self.repo.root, &file.path) {
            Ok(()) => {
                self.refresh();
                self.message = format!("{} 取消暂存啦喵~", file.path);
                self.message_error = false;
            }
            Err(error) => {
                self.message = error;
                self.message_error = true;
            }
        }
    }

    fn begin_commit(&mut self) {
        if self.repo.staged_count() == 0 {
            self.message = "先按 s 暂存一些改动，再写提交信息喵~".to_string();
            self.message_error = true;
            return;
        }
        self.commit_input = Some(TextInput::new(""));
        self.message = "写提交信息，Enter 提交，Esc 取消喵~".to_string();
        self.message_error = false;
    }

    fn submit_commit(&mut self) {
        let Some(input) = self.commit_input.as_ref() else {
            return;
        };
        let message = input.value.trim().to_string();
        if message.is_empty() {
            self.message = "提交信息不能空着喵~".to_string();
            self.message_error = true;
            return;
        }
        match local::commit(&self.repo.root, &message) {
            Ok(()) => {
                self.commit_input = None;
                self.refresh();
                self.message = "提交好啦喵~".to_string();
                self.message_error = false;
            }
            Err(error) => {
                self.message = error;
                self.message_error = true;
            }
        }
    }

    fn handle_commit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => {
                self.submit_commit();
                return;
            }
            KeyCode::Esc => {
                self.commit_input = None;
                self.message = "取消提交啦喵~".to_string();
                self.message_error = false;
                return;
            }
            _ => {}
        }
        let Some(input) = self.commit_input.as_mut() else {
            return;
        };
        match key.code {
            KeyCode::Backspace => input.backspace(),
            KeyCode::Delete => input.delete(),
            KeyCode::Left => input.left(),
            KeyCode::Right => input.right(),
            KeyCode::Home => input.home(),
            KeyCode::End => input.end(),
            KeyCode::Char(ch) => input.insert(ch),
            _ => {}
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
            return;
        }
        if self.commit_input.is_some() {
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('s') | KeyCode::Char('S'))
            {
                self.submit_commit();
            } else {
                self.handle_commit_key(key);
            }
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Tab | KeyCode::Right => self.cycle_pane(),
            KeyCode::BackTab | KeyCode::Left => self.cycle_pane(),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::PageUp => self.move_selection(-10),
            KeyCode::PageDown => self.move_selection(10),
            KeyCode::Home | KeyCode::Char('g') => self.set_selection_edge(false),
            KeyCode::End | KeyCode::Char('G') => self.set_selection_edge(true),
            KeyCode::Char('s') => self.stage_selected(),
            KeyCode::Char('u') => self.unstage_selected(),
            KeyCode::Char('c') => self.begin_commit(),
            KeyCode::Char('r') => self.refresh(),
            _ => {}
        }
    }

    pub fn handle_paste(&mut self, text: &str) {
        if let Some(input) = self.commit_input.as_mut() {
            for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                input.insert(ch);
            }
        }
    }
}

pub fn run(
    terminal: &mut DefaultTerminal,
    path: &Path,
    theme_override: Option<String>,
) -> Result<()> {
    let mut app = match LocalApp::new(path, theme_override) {
        Ok(app) => app,
        Err(error) => anyhow::bail!("{error}"),
    };
    loop {
        terminal.draw(|frame| local_ui::draw(frame, &app))?;
        if app.should_quit {
            break;
        }
        if event::poll(Duration::from_millis(120))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                        app.handle_key(key);
                    }
                }
                Event::Paste(text) => app.handle_paste(&text),
                _ => {}
            }
        }
    }
    Ok(())
}
