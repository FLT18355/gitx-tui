use crate::app::TextInput;
use crate::config::{Config, Paths};
use crate::format;
use crate::local::{self, LocalRepo};
use crate::local_ops;
use crate::local_ui;
use crate::theme::{FLAVORS, Theme};
use anyhow::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::DefaultTerminal;
use ratatui::layout::Rect;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalPane {
    Status,
    Diff,
    Commits,
    Branches,
    Tags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Commit,
    Amend,
    Tag,
    Branch,
    Discard,
}

pub struct Prompt {
    pub kind: PromptKind,
    pub input: TextInput,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalLayout {
    pub branches: Rect,
    pub tags: Rect,
    pub status: Rect,
    pub diff: Rect,
    pub commits: Rect,
}

struct LocalMsg {
    result: Result<String, String>,
}

pub struct LocalApp {
    pub theme: Theme,
    pub repo: LocalRepo,
    pub layout: LocalLayout,
    pub pane: LocalPane,
    pub diff_context: LocalPane,
    pub file_selected: usize,
    pub commit_selected: usize,
    pub branch_selected: usize,
    pub tag_selected: usize,
    pub diff_text: String,
    pub diff_title: String,
    pub diff_staged: bool,
    pub diff_scroll: u16,
    pub diff_scroll_max: u16,
    pub status_rows: usize,
    pub commit_rows: usize,
    pub branch_rows: usize,
    pub tag_rows: usize,
    pub message: String,
    pub message_error: bool,
    pub prompt: Option<Prompt>,
    pub busy: Option<String>,
    pub started_at: Instant,
    pub should_quit: bool,
    tx: Sender<LocalMsg>,
    rx: Receiver<LocalMsg>,
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
        let (tx, rx) = mpsc::channel();
        let message = if repo.files.is_empty() {
            "工作区干干净净，没有改动喵~".to_string()
        } else {
            format!(
                "读到 {} 个改动、{} 个提交喵~",
                repo.files.len(),
                repo.commits.len()
            )
        };
        let mut app = Self {
            theme,
            repo,
            layout: LocalLayout::default(),
            pane: LocalPane::Status,
            diff_context: LocalPane::Status,
            file_selected: 0,
            commit_selected: 0,
            branch_selected: 0,
            tag_selected: 0,
            diff_text: String::new(),
            diff_title: "差异喵~".to_string(),
            diff_staged: false,
            diff_scroll: 0,
            diff_scroll_max: 0,
            status_rows: 8,
            commit_rows: 4,
            branch_rows: 6,
            tag_rows: 6,
            message,
            message_error: false,
            prompt: None,
            busy: None,
            started_at: Instant::now(),
            should_quit: false,
            tx,
            rx,
        };
        app.refresh_diff();
        Ok(app)
    }

    pub fn refresh(&mut self) {
        match LocalRepo::open(&self.repo.root) {
            Ok(repo) => {
                self.repo = repo;
                self.clamp_selection();
                self.refresh_diff();
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
        self.tag_selected = self
            .tag_selected
            .min(self.repo.tags.len().saturating_sub(1));
    }

    fn selected_file(&self) -> Option<&local::StatusFile> {
        self.repo.files.get(self.file_selected)
    }

    fn selected_commit(&self) -> Option<&local::Commit> {
        self.repo.commits.get(self.commit_selected)
    }

    fn selected_branch(&self) -> Option<&local::Branch> {
        self.repo.branches.get(self.branch_selected)
    }

    fn selected_tag(&self) -> Option<&local::Tag> {
        self.repo.tags.get(self.tag_selected)
    }

    fn refresh_diff(&mut self) {
        if self.pane != LocalPane::Diff {
            self.diff_context = self.pane;
        }
        let root = self.repo.root.clone();
        let result = match self.diff_context {
            LocalPane::Status => match self.selected_file().cloned() {
                Some(file) => {
                    let staged = if self.diff_staged {
                        file.staged()
                    } else {
                        !file.unstaged() && file.staged()
                    };
                    let label = if staged { "暂存" } else { "未暂存" };
                    self.diff_title =
                        format!("差异喵~  {} ({label})", format::truncate(&file.path, 40));
                    local_ops::file_diff(&root, &file.path, staged)
                }
                None => {
                    self.diff_title = "差异喵~".to_string();
                    Ok(String::new())
                }
            },
            LocalPane::Commits => match self.selected_commit().cloned() {
                Some(commit) => {
                    self.diff_title = format!("提交详情喵~  {}", commit.short);
                    local_ops::commit_diff(&root, &commit.hash)
                }
                None => {
                    self.diff_title = "提交详情喵~".to_string();
                    Ok(String::new())
                }
            },
            LocalPane::Branches => match self.selected_branch().cloned() {
                Some(branch) => {
                    self.diff_title = format!("分支记录喵~  {}", branch.name);
                    local_ops::ref_log(&root, &branch.name)
                }
                None => {
                    self.diff_title = "分支记录喵~".to_string();
                    Ok(String::new())
                }
            },
            LocalPane::Tags => match self.selected_tag().cloned() {
                Some(tag) => {
                    self.diff_title = format!("标签详情喵~  {}", tag.name);
                    local_ops::tag_info(&root, &tag.name)
                }
                None => {
                    self.diff_title = "标签详情喵~".to_string();
                    Ok(String::new())
                }
            },
            LocalPane::Diff => Ok(String::new()),
        };
        self.diff_text = match result {
            Ok(text) if text.is_empty() => "这里没有可显示的内容喵~".to_string(),
            Ok(text) => text,
            Err(error) => format!("读差异失败喵~：{error}"),
        };
        self.diff_scroll = 0;
        self.diff_scroll_max = 0;
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
            LocalPane::Tags => {
                if self.repo.tags.is_empty() {
                    return;
                }
                self.tag_selected = (self.tag_selected as i32 + delta)
                    .clamp(0, self.repo.tags.len() as i32 - 1)
                    as usize;
            }
            LocalPane::Diff => {
                self.scroll_diff(delta * 3);
                return;
            }
        }
        self.refresh_diff();
    }

    fn scroll_diff(&mut self, delta: i32) {
        self.diff_scroll =
            (self.diff_scroll as i32 + delta).clamp(0, self.diff_scroll_max as i32) as u16;
    }

    fn move_page(&mut self, direction: i32) {
        let rows = match self.pane {
            LocalPane::Status => self.status_rows,
            LocalPane::Commits => self.commit_rows,
            LocalPane::Branches => self.branch_rows,
            LocalPane::Tags => self.tag_rows,
            LocalPane::Diff => {
                self.scroll_diff(direction * self.diff_scroll_max.max(1) as i32);
                return;
            }
        };
        self.move_selection(direction * rows.max(1) as i32);
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
            LocalPane::Tags => {
                self.tag_selected = if end {
                    self.repo.tags.len().saturating_sub(1)
                } else {
                    0
                };
            }
            LocalPane::Diff => {
                self.diff_scroll = if end { self.diff_scroll_max } else { 0 };
                return;
            }
        }
        self.refresh_diff();
    }

    fn cycle_pane(&mut self, direction: i32) {
        let order = [
            LocalPane::Status,
            LocalPane::Diff,
            LocalPane::Commits,
            LocalPane::Branches,
            LocalPane::Tags,
        ];
        let current = order
            .iter()
            .position(|pane| *pane == self.pane)
            .unwrap_or(0) as i32;
        let next = (current + direction).rem_euclid(order.len() as i32) as usize;
        self.pane = order[next];
        if self.pane != LocalPane::Diff {
            self.diff_context = self.pane;
        }
        self.refresh_diff();
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
        match local_ops::stage(&self.repo.root, &file.path) {
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
        match local_ops::unstage(&self.repo.root, &file.path) {
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

    fn stage_all(&mut self) {
        match local_ops::stage_all(&self.repo.root) {
            Ok(()) => {
                self.refresh();
                self.message = "所有改动都暂存好啦喵~".to_string();
                self.message_error = false;
            }
            Err(error) => {
                self.message = error;
                self.message_error = true;
            }
        }
    }

    fn unstage_all(&mut self) {
        match local_ops::unstage_all(&self.repo.root) {
            Ok(()) => {
                self.refresh();
                self.message = "所有暂存都取消啦喵~".to_string();
                self.message_error = false;
            }
            Err(error) => {
                self.message = error;
                self.message_error = true;
            }
        }
    }

    fn checkout_selected_branch(&mut self) {
        let Some(branch) = self.selected_branch().cloned() else {
            return;
        };
        if branch.current {
            self.message = "已经在当前分支上啦喵~".to_string();
            self.message_error = false;
            return;
        }
        match local_ops::checkout(&self.repo.root, &branch.name) {
            Ok(()) => {
                self.refresh();
                self.message = format!("切到 {} 分支啦喵~", branch.name);
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
        self.prompt = Some(Prompt {
            kind: PromptKind::Commit,
            input: TextInput::new(""),
            target: None,
        });
        self.message = "写提交信息，Enter 提交，Esc 取消喵~".to_string();
        self.message_error = false;
    }

    fn begin_amend(&mut self) {
        let Some(commit) = self.repo.commits.first() else {
            self.message = "还没有提交可以 amend 喵~".to_string();
            self.message_error = true;
            return;
        };
        self.prompt = Some(Prompt {
            kind: PromptKind::Amend,
            input: TextInput::new(commit.summary.clone()),
            target: None,
        });
        self.message = "修改最后一条提交信息，Enter 确认，Esc 取消喵~".to_string();
        self.message_error = false;
    }

    fn begin_tag(&mut self) {
        self.prompt = Some(Prompt {
            kind: PromptKind::Tag,
            input: TextInput::new(""),
            target: None,
        });
        self.message = "写标签名，Enter 创建，Esc 取消喵~".to_string();
        self.message_error = false;
    }

    fn begin_branch(&mut self) {
        self.prompt = Some(Prompt {
            kind: PromptKind::Branch,
            input: TextInput::new(""),
            target: None,
        });
        self.message = "写新分支名，Enter 创建并切换，Esc 取消喵~".to_string();
        self.message_error = false;
    }

    fn begin_discard(&mut self) {
        let Some(file) = self.selected_file().cloned() else {
            self.message = "先选中一个文件再丢弃喵~".to_string();
            self.message_error = true;
            return;
        };
        self.prompt = Some(Prompt {
            kind: PromptKind::Discard,
            input: TextInput::new(""),
            target: Some(file.path),
        });
        self.message = "这条改动会真实丢弃，Enter 确认，Esc 取消喵~".to_string();
        self.message_error = false;
    }

    fn submit_prompt(&mut self) {
        let Some(prompt) = self.prompt.as_ref() else {
            return;
        };
        let kind = prompt.kind;
        let target = prompt.target.clone();
        let value = prompt.input.value.trim().to_string();
        if !matches!(kind, PromptKind::Discard) && value.is_empty() {
            self.message = match kind {
                PromptKind::Commit | PromptKind::Amend => "提交信息不能空着喵~".to_string(),
                PromptKind::Tag => "标签名不能空着喵~".to_string(),
                PromptKind::Branch => "分支名不能空着喵~".to_string(),
                PromptKind::Discard => String::new(),
            };
            self.message_error = true;
            return;
        }
        let result = match kind {
            PromptKind::Commit => local_ops::commit(&self.repo.root, &value),
            PromptKind::Amend => local_ops::amend(&self.repo.root, &value),
            PromptKind::Tag => local_ops::create_tag(&self.repo.root, &value),
            PromptKind::Branch => local_ops::create_branch(&self.repo.root, &value),
            PromptKind::Discard => match target.as_deref() {
                Some(path) => local_ops::discard(&self.repo.root, path),
                None => Err("没有选中要丢弃的文件喵~".to_string()),
            },
        };
        match result {
            Ok(()) => {
                self.prompt = None;
                self.refresh();
                self.message = match kind {
                    PromptKind::Commit => "提交好啦喵~".to_string(),
                    PromptKind::Amend => "最后一条提交改好啦喵~".to_string(),
                    PromptKind::Tag => format!("标签 {value} 创建好啦喵~"),
                    PromptKind::Branch => format!("分支 {value} 建好并切过去啦喵~"),
                    PromptKind::Discard => {
                        format!("{} 的改动丢弃啦喵~", target.unwrap_or_default())
                    }
                };
                self.message_error = false;
            }
            Err(error) => {
                self.message = error;
                self.message_error = true;
            }
        }
    }

    fn stash_save(&mut self) {
        match local_ops::stash_save(&self.repo.root) {
            Ok(message) => {
                self.refresh();
                self.message = message;
                self.message_error = false;
            }
            Err(error) => {
                self.message = error;
                self.message_error = true;
            }
        }
    }

    fn stash_pop(&mut self) {
        match local_ops::stash_pop(&self.repo.root) {
            Ok(message) => {
                self.refresh();
                self.message = message;
                self.message_error = false;
            }
            Err(error) => {
                self.message = error;
                self.message_error = true;
            }
        }
    }

    fn toggle_diff_staged(&mut self) {
        if self.diff_context != LocalPane::Status {
            return;
        }
        self.diff_staged = !self.diff_staged;
        self.refresh_diff();
    }

    fn start_remote<F>(&mut self, label: &str, work: F)
    where
        F: FnOnce() -> Result<String, String> + Send + 'static,
    {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some(label.to_string());
        self.message = label.to_string();
        self.message_error = false;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(LocalMsg { result: work() });
        });
    }

    fn push(&mut self) {
        let root = self.repo.root.clone();
        let branch = self.repo.branch.clone();
        self.start_remote("正在推送到远端喵~...", move || {
            local_ops::push(&root, &branch)
        });
    }

    fn push_tags(&mut self) {
        let root = self.repo.root.clone();
        self.start_remote("正在推送标签喵~...", move || {
            local_ops::push_tags(&root)
        });
    }

    fn fetch(&mut self) {
        let root = self.repo.root.clone();
        self.start_remote("正在取远端更新喵~...", move || {
            local_ops::fetch(&root)
        });
    }

    fn pull(&mut self) {
        let root = self.repo.root.clone();
        self.start_remote("正在拉取远端喵~...", move || local_ops::pull(&root));
    }

    fn poll_messages(&mut self) {
        while let Ok(message) = self.rx.try_recv() {
            self.busy = None;
            match message.result {
                Ok(text) => {
                    self.refresh();
                    self.message = if text.is_empty() {
                        "操作完成啦喵~".to_string()
                    } else {
                        text
                    };
                    self.message_error = false;
                }
                Err(error) => {
                    self.message = error;
                    self.message_error = true;
                }
            }
        }
    }

    fn handle_prompt_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => {
                self.submit_prompt();
                return;
            }
            KeyCode::Esc => {
                let kind = self.prompt.as_ref().map(|prompt| prompt.kind);
                self.prompt = None;
                self.message = match kind {
                    Some(PromptKind::Commit) => "取消提交啦喵~".to_string(),
                    Some(PromptKind::Amend) => "取消修改提交啦喵~".to_string(),
                    Some(PromptKind::Tag) => "取消创建标签啦喵~".to_string(),
                    Some(PromptKind::Branch) => "取消创建分支啦喵~".to_string(),
                    Some(PromptKind::Discard) => "取消丢弃啦喵~".to_string(),
                    None => String::new(),
                };
                self.message_error = false;
                return;
            }
            _ => {}
        }
        let Some(prompt) = self.prompt.as_mut() else {
            return;
        };
        if prompt.kind == PromptKind::Discard {
            return;
        }
        match key.code {
            KeyCode::Backspace => prompt.input.backspace(),
            KeyCode::Delete => prompt.input.delete(),
            KeyCode::Left => prompt.input.left(),
            KeyCode::Right => prompt.input.right(),
            KeyCode::Home => prompt.input.home(),
            KeyCode::End => prompt.input.end(),
            KeyCode::Char(ch) => prompt.input.insert(ch),
            _ => {}
        }
    }

    fn pane_at(&self, x: u16, y: u16) -> Option<LocalPane> {
        let rects = [
            (LocalPane::Branches, self.layout.branches),
            (LocalPane::Tags, self.layout.tags),
            (LocalPane::Status, self.layout.status),
            (LocalPane::Diff, self.layout.diff),
            (LocalPane::Commits, self.layout.commits),
        ];
        rects.into_iter().find_map(|(pane, rect)| {
            (rect.width > 0
                && rect.height > 0
                && x >= rect.x
                && x < rect.x.saturating_add(rect.width)
                && y >= rect.y
                && y < rect.y.saturating_add(rect.height))
            .then_some(pane)
        })
    }

    fn focus_pane(&mut self, pane: LocalPane) {
        self.pane = pane;
        if pane != LocalPane::Diff {
            self.diff_context = pane;
        }
    }

    fn select_clicked(&mut self, pane: LocalPane, y: u16) -> bool {
        let (rect, rows, row_height, len) = match pane {
            LocalPane::Status => (
                self.layout.status,
                self.status_rows.max(1),
                1,
                self.repo.files.len(),
            ),
            LocalPane::Commits => (
                self.layout.commits,
                self.commit_rows.max(1),
                2,
                self.repo.commits.len(),
            ),
            LocalPane::Branches => (
                self.layout.branches,
                self.branch_rows.max(1),
                1,
                self.repo.branches.len(),
            ),
            LocalPane::Tags => (
                self.layout.tags,
                self.tag_rows.max(1),
                1,
                self.repo.tags.len(),
            ),
            LocalPane::Diff => return false,
        };
        if len == 0 || y < rect.y {
            return false;
        }
        let offset = ((y - rect.y) / row_height) as usize;
        let selected = match pane {
            LocalPane::Status => self.file_selected,
            LocalPane::Commits => self.commit_selected,
            LocalPane::Branches => self.branch_selected,
            LocalPane::Tags => self.tag_selected,
            LocalPane::Diff => return false,
        };
        let start = (selected / rows) * rows;
        let index = start + offset;
        if index >= len {
            return false;
        }
        match pane {
            LocalPane::Status => self.file_selected = index,
            LocalPane::Commits => self.commit_selected = index,
            LocalPane::Branches => self.branch_selected = index,
            LocalPane::Tags => self.tag_selected = index,
            LocalPane::Diff => {}
        }
        self.refresh_diff();
        true
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.prompt.is_some() {
            return;
        }
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(pane) = self.pane_at(mouse.column, mouse.row) {
                    let previous = self.pane;
                    self.focus_pane(pane);
                    let selected = self.select_clicked(pane, mouse.row);
                    if self.pane != previous && pane != LocalPane::Diff && !selected {
                        self.refresh_diff();
                    }
                }
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let Some(pane) = self.pane_at(mouse.column, mouse.row) else {
                    return;
                };
                let previous = self.pane;
                self.focus_pane(pane);
                let delta = if mouse.kind == MouseEventKind::ScrollUp {
                    -1
                } else {
                    1
                };
                if pane == LocalPane::Diff {
                    self.scroll_diff(delta * 3);
                } else {
                    self.move_selection(delta);
                    if pane != previous {
                        self.refresh_diff();
                    }
                }
            }
            _ => {}
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
            return;
        }
        if self.prompt.is_some() {
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('s') | KeyCode::Char('S'))
            {
                self.submit_prompt();
            } else {
                self.handle_prompt_key(key);
            }
            return;
        }
        if self.busy.is_some() && !matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Tab | KeyCode::Right => self.cycle_pane(1),
            KeyCode::BackTab | KeyCode::Left => self.cycle_pane(-1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::PageUp | KeyCode::Char('[') => self.move_page(-1),
            KeyCode::PageDown | KeyCode::Char(']') => self.move_page(1),
            KeyCode::Home | KeyCode::Char('g') => self.set_selection_edge(false),
            KeyCode::End | KeyCode::Char('G') => self.set_selection_edge(true),
            KeyCode::Char('s') => self.stage_selected(),
            KeyCode::Char('u') => self.unstage_selected(),
            KeyCode::Char('S') => self.stage_all(),
            KeyCode::Char('U') => self.unstage_all(),
            KeyCode::Char('d') => self.toggle_diff_staged(),
            KeyCode::Char('c') => self.begin_commit(),
            KeyCode::Char('A') => self.begin_amend(),
            KeyCode::Char('t') => self.begin_tag(),
            KeyCode::Char('b') => self.begin_branch(),
            KeyCode::Char('x') => self.begin_discard(),
            KeyCode::Char('z') => self.stash_save(),
            KeyCode::Char('Z') => self.stash_pop(),
            KeyCode::Char('p') => self.push(),
            KeyCode::Char('P') => self.push_tags(),
            KeyCode::Char('f') => self.fetch(),
            KeyCode::Char('L') => self.pull(),
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Enter => {
                if self.pane == LocalPane::Branches {
                    self.checkout_selected_branch();
                } else {
                    self.refresh_diff();
                }
            }
            _ => {}
        }
    }

    pub fn handle_paste(&mut self, text: &str) {
        if let Some(prompt) = self.prompt.as_mut() {
            for ch in text.chars().filter(|ch| *ch != '\n' && *ch != '\r') {
                prompt.input.insert(ch);
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
        app.poll_messages();
        terminal.draw(|frame| local_ui::draw(frame, &mut app))?;
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
                Event::Mouse(mouse) => app.handle_mouse(mouse),
                _ => {}
            }
        }
    }
    Ok(())
}
