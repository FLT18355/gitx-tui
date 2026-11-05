use crate::app::TextInput;
use crate::config::{Config, Paths};
use crate::format;
use crate::icons;
use crate::theme::{FLAVORS, Theme};
use anyhow::Result;
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use ratatui::layout::Rect;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    Files,
    Outline,
    Preview,
}

impl Tab {
    fn next(self) -> Self {
        match self {
            Self::Files => Self::Outline,
            Self::Outline => Self::Preview,
            Self::Preview => Self::Files,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sort {
    Path,
    Size,
    Modified,
}

impl Sort {
    fn next(self) -> Self {
        match self {
            Self::Path => Self::Size,
            Self::Size => Self::Modified,
            Self::Modified => Self::Path,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Path => "路径",
            Self::Size => "大小",
            Self::Modified => "时间",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatusKind {
    Info,
    Ok,
    Error,
}

pub(crate) struct DocFile {
    pub(crate) path: PathBuf,
    pub(crate) rel: String,
    pub(crate) size: u64,
    pub(crate) modified: Option<SystemTime>,
}

pub(crate) struct Heading {
    pub(crate) level: usize,
    pub(crate) title: String,
    pub(crate) line: usize,
}

pub(crate) struct LoadedDoc {
    pub(crate) path: PathBuf,
    pub(crate) rel: String,
    pub(crate) content: String,
    pub(crate) headings: Vec<Heading>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hit {
    FileRow(usize),
    OutlineRow(usize),
    FocusTab(Tab),
    FilesPanel,
    OutlinePanel,
    PreviewPanel,
    FilterRow,
    SearchRow,
    CloseHelp,
}

pub(crate) struct MdApp {
    pub(crate) root: PathBuf,
    pub(crate) files: Vec<DocFile>,
    pub(crate) visible: Vec<usize>,
    pub(crate) selected: usize,
    pub(crate) offset: usize,
    pub(crate) filter: TextInput,
    pub(crate) filter_editing: bool,
    pub(crate) search: TextInput,
    pub(crate) search_editing: bool,
    pub(crate) search_query: String,
    pub(crate) search_matches: Vec<usize>,
    pub(crate) search_index: usize,
    pub(crate) focus: Tab,
    pub(crate) pane: Tab,
    pub(crate) sort: Sort,
    pub(crate) doc: Option<LoadedDoc>,
    pub(crate) outline_selected: usize,
    pub(crate) outline_offset: usize,
    pub(crate) preview_scroll: u16,
    pub(crate) preview_scroll_max: u16,
    pub(crate) preview_viewport: u16,
    pub(crate) preview_width: usize,
    pub(crate) raw: bool,
    pub(crate) fullscreen: bool,
    pub(crate) theme: Theme,
    pub(crate) paths: Paths,
    pub(crate) show_help: bool,
    pub(crate) status: String,
    pub(crate) status_kind: StatusKind,
    pub(crate) should_quit: bool,
    pub(crate) editor: Option<PathBuf>,
    pub(crate) hits: Vec<(Rect, Hit)>,
    pub(crate) hover: Option<Hit>,
    pub(crate) files_rect: Rect,
    pub(crate) outline_rect: Rect,
    pub(crate) preview_rect: Rect,
}

impl MdApp {
    pub(crate) fn new(paths: Paths, root: PathBuf, theme_override: Option<String>) -> Self {
        let config = Config::load(&paths);
        let theme_name = theme_override
            .filter(|value| Theme::exists(&paths, value))
            .unwrap_or_else(|| config.theme.clone());
        let mut app = Self {
            root,
            files: Vec::new(),
            visible: Vec::new(),
            selected: 0,
            offset: 0,
            filter: TextInput::new(""),
            filter_editing: false,
            search: TextInput::new(""),
            search_editing: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_index: 0,
            focus: Tab::Files,
            pane: Tab::Files,
            sort: Sort::Path,
            doc: None,
            outline_selected: 0,
            outline_offset: 0,
            preview_scroll: 0,
            preview_scroll_max: 0,
            preview_viewport: 10,
            preview_width: 60,
            raw: false,
            fullscreen: false,
            theme: Theme::load(&paths, &theme_name),
            paths,
            show_help: false,
            status: String::new(),
            status_kind: StatusKind::Info,
            should_quit: false,
            editor: None,
            hits: Vec::new(),
            hover: None,
            files_rect: Rect::default(),
            outline_rect: Rect::default(),
            preview_rect: Rect::default(),
        };
        app.rescan();
        app
    }

    pub(crate) fn set_ok(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.status_kind = StatusKind::Ok;
    }

    pub(crate) fn set_error(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.status_kind = StatusKind::Error;
    }

    pub(crate) fn rel_label(&self) -> String {
        self.doc
            .as_ref()
            .map(|doc| doc.rel.clone())
            .unwrap_or_else(|| "没有打开文件喵~".to_string())
    }

    fn rescan(&mut self) {
        self.clear_search();
        let keep = self.doc.as_ref().map(|doc| doc.rel.clone());
        match scan_docs(&self.root) {
            Ok(files) => {
                self.files = files;
                if self.files.is_empty() {
                    self.set_error(format!(
                        "{} 里没有找到 Markdown 文件喵~",
                        format::display_path(&self.root)
                    ));
                } else {
                    self.set_ok(format!(
                        "{} 扫到 {} 个 Markdown 文件喵~",
                        icons::SEARCH,
                        self.files.len()
                    ));
                }
                self.refresh_visible();
                self.restore_selection(keep.as_deref());
                if let Some(path) = self.doc.as_ref().map(|doc| doc.path.clone())
                    && let Ok(content) = fs::read_to_string(&path)
                    && let Some(doc) = self.doc.as_mut()
                {
                    doc.headings = parse_headings(&content);
                    doc.content = content;
                }
                if self.doc.is_none() && !self.visible.is_empty() {
                    self.selected = 0;
                    self.open_selected(false);
                }
            }
            Err(error) => {
                self.files.clear();
                self.visible.clear();
                self.doc = None;
                self.set_error(error);
            }
        }
    }

    fn restore_selection(&mut self, keep: Option<&str>) {
        if let Some(rel) = keep
            && let Some(position) = self
                .visible
                .iter()
                .position(|index| self.files[*index].rel == rel)
        {
            self.selected = position;
            return;
        }
        self.selected = self.selected.min(self.visible.len().saturating_sub(1));
    }

    fn refresh_visible(&mut self) {
        let query = self.filter.value.trim().to_lowercase();
        let mut ids: Vec<usize> = self
            .files
            .iter()
            .enumerate()
            .filter(|(_, file)| query.is_empty() || file.rel.to_lowercase().contains(&query))
            .map(|(index, _)| index)
            .collect();
        let files = &self.files;
        match self.sort {
            Sort::Path => ids.sort_by(|a, b| files[*a].rel.cmp(&files[*b].rel)),
            Sort::Size => ids.sort_by(|a, b| files[*b].size.cmp(&files[*a].size)),
            Sort::Modified => ids.sort_by(|a, b| files[*b].modified.cmp(&files[*a].modified)),
        }
        self.visible = ids;
        if self.selected >= self.visible.len() {
            self.selected = self.visible.len().saturating_sub(1);
        }
    }

    fn open_selected(&mut self, focus_preview: bool) {
        let Some(index) = self.visible.get(self.selected).copied() else {
            return;
        };
        let (path, rel) = (
            self.files[index].path.clone(),
            self.files[index].rel.clone(),
        );
        match fs::read_to_string(&path) {
            Ok(content) => {
                let headings = parse_headings(&content);
                let lines = content.lines().count();
                self.preview_scroll = 0;
                self.preview_scroll_max = 0;
                self.outline_selected = 0;
                self.outline_offset = 0;
                self.status = format!(
                    "{} {} · {} 行 · {} 个标题喵~",
                    icons::BOOK,
                    rel,
                    lines,
                    headings.len()
                );
                self.status_kind = StatusKind::Ok;
                self.doc = Some(LoadedDoc {
                    path,
                    rel,
                    content,
                    headings,
                });
                self.clear_search();
                if focus_preview {
                    self.focus = Tab::Preview;
                }
            }
            Err(error) => self.set_error(format!("读不了 {rel} 喵~：{error}")),
        }
    }

    fn reload_current(&mut self) {
        self.rescan();
    }

    fn move_selection(&mut self, delta: isize) {
        if self.visible.is_empty() {
            return;
        }
        let last = self.visible.len() - 1;
        let next = (self.selected as isize + delta).clamp(0, last as isize) as usize;
        if next != self.selected {
            self.selected = next;
            self.open_selected(false);
        }
    }

    fn move_outline(&mut self, delta: isize) {
        let count = self.doc.as_ref().map(|doc| doc.headings.len()).unwrap_or(0);
        if count == 0 {
            return;
        }
        let next = (self.outline_selected as isize + delta).clamp(0, count as isize - 1) as usize;
        self.outline_selected = next;
        if !self.raw {
            self.jump_outline();
        }
    }

    fn jump_outline(&mut self) {
        let Some(doc) = self.doc.as_ref() else {
            return;
        };
        let Some(heading) = doc.headings.get(self.outline_selected) else {
            return;
        };
        let scroll = if self.raw {
            heading.line
        } else {
            let byte = doc
                .content
                .lines()
                .take(heading.line)
                .map(|line| line.len() + 1)
                .sum::<usize>()
                .min(doc.content.len());
            crate::ui::markdown_lines(
                &doc.content[..byte],
                self.preview_width.max(20),
                &self.theme,
            )
            .len()
        };
        self.preview_scroll = scroll.min(u16::MAX as usize) as u16;
    }

    fn cycle_theme(&mut self) {
        let current = self.theme.flavor.clone();
        let index = FLAVORS
            .iter()
            .position(|flavor| *flavor == current)
            .unwrap_or(0);
        let next = FLAVORS[(index + 1) % FLAVORS.len()];
        self.theme = Theme::load(&self.paths, next);
        let mut config = Config::load(&self.paths);
        config.theme = next.to_string();
        match config.save(&self.paths) {
            Ok(()) => self.set_ok(format!(
                "{} 换成 {} 主题啦喵~ 已写进配置",
                icons::PALETTE,
                self.theme.display_name()
            )),
            Err(error) => self.set_error(format!("主题换了，但保存不了配置喵~：{error}")),
        }
    }

    fn scroll_preview(&mut self, delta: isize) {
        let next = (self.preview_scroll as isize + delta).clamp(0, self.preview_scroll_max as isize)
            as u16;
        self.preview_scroll = next;
    }

    fn searchable_lines(&self) -> Vec<String> {
        let Some(doc) = self.doc.as_ref() else {
            return Vec::new();
        };
        if self.raw {
            return doc.content.lines().map(str::to_string).collect();
        }
        crate::ui::markdown_lines(&doc.content, self.preview_width.max(20), &self.theme)
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    pub(crate) fn start_search(&mut self) {
        if self.doc.is_none() {
            return;
        }
        self.search = TextInput::new(self.search_query.clone());
        self.search_editing = true;
        self.focus = Tab::Preview;
    }

    fn confirm_search(&mut self) {
        self.search_editing = false;
        self.search_query = self.search.value.trim().to_string();
        self.refresh_search();
    }

    fn refresh_search(&mut self) {
        self.search_matches.clear();
        self.search_index = 0;
        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            return;
        }
        for (index, line) in self.searchable_lines().iter().enumerate() {
            if line.to_lowercase().contains(&query) {
                self.search_matches.push(index);
            }
        }
        if self.search_matches.is_empty() {
            self.set_error(format!("没有找到 \"{}\" 喵~", self.search_query));
            return;
        }
        self.jump_search(0);
    }

    fn jump_search(&mut self, index: usize) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_index = index % self.search_matches.len();
        let line = self.search_matches[self.search_index];
        self.preview_scroll = line.saturating_sub(2).min(u16::MAX as usize) as u16;
        self.set_ok(format!(
            "{} 搜索 \"{}\" {}/{} 喵~",
            icons::SEARCH,
            self.search_query,
            self.search_index + 1,
            self.search_matches.len()
        ));
    }

    fn next_search(&mut self, delta: isize) {
        if self.search_matches.is_empty() {
            return;
        }
        let len = self.search_matches.len() as isize;
        let next = (self.search_index as isize + delta).rem_euclid(len) as usize;
        self.jump_search(next);
    }

    fn clear_search(&mut self) {
        self.search_editing = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.search_index = 0;
        self.search = TextInput::new("");
    }

    fn page_step(&self) -> usize {
        if self.focus == Tab::Preview {
            self.preview_viewport.saturating_sub(2).max(5) as usize
        } else {
            10
        }
    }

    fn next_focus(&mut self) {
        let next = if self.fullscreen {
            Tab::Preview
        } else {
            self.focus.next()
        };
        self.focus = next;
        if next != Tab::Preview {
            self.pane = next;
        }
    }

    fn prev_focus(&mut self) {
        let next = if self.fullscreen {
            Tab::Preview
        } else {
            match self.focus {
                Tab::Files => Tab::Preview,
                Tab::Outline => Tab::Files,
                Tab::Preview => Tab::Outline,
            }
        };
        self.focus = next;
        if next != Tab::Preview {
            self.pane = next;
        }
    }

    fn down(&mut self, step: usize) {
        match self.focus {
            Tab::Files => self.move_selection(step as isize),
            Tab::Outline => self.move_outline(step as isize),
            Tab::Preview => self.scroll_preview(step as isize),
        }
    }

    fn up(&mut self, step: usize) {
        match self.focus {
            Tab::Files => self.move_selection(-(step as isize)),
            Tab::Outline => self.move_outline(-(step as isize)),
            Tab::Preview => self.scroll_preview(-(step as isize)),
        }
    }

    fn top(&mut self) {
        match self.focus {
            Tab::Files => {
                self.selected = 0;
                self.open_selected(false);
            }
            Tab::Outline => {
                self.outline_selected = 0;
                if !self.raw {
                    self.jump_outline();
                }
            }
            Tab::Preview => self.preview_scroll = 0,
        }
    }

    fn bottom(&mut self) {
        match self.focus {
            Tab::Files => {
                self.selected = self.visible.len().saturating_sub(1);
                self.open_selected(false);
            }
            Tab::Outline => {
                self.outline_selected = self
                    .doc
                    .as_ref()
                    .map(|doc| doc.headings.len().saturating_sub(1))
                    .unwrap_or(0);
                if !self.raw {
                    self.jump_outline();
                }
            }
            Tab::Preview => self.preview_scroll = self.preview_scroll_max,
        }
    }

    fn activate(&mut self) {
        match self.focus {
            Tab::Files => self.open_selected(true),
            Tab::Outline => {
                self.jump_outline();
                self.focus = Tab::Preview;
            }
            Tab::Preview => {}
        }
    }

    fn toggle_raw(&mut self) {
        if self.doc.is_none() {
            return;
        }
        self.raw = !self.raw;
        self.preview_scroll = 0;
        self.clear_search();
        self.set_ok(if self.raw {
            "切到源码模式啦喵~ m 回到渲染喵~".to_string()
        } else {
            "切回 Markdown 渲染啦喵~".to_string()
        });
    }

    fn toggle_fullscreen(&mut self) {
        if self.doc.is_none() {
            return;
        }
        self.fullscreen = !self.fullscreen;
        self.focus = Tab::Preview;
        self.clear_search();
    }

    fn escape(&mut self) {
        if !self.search_query.is_empty() || self.search_editing {
            self.clear_search();
            return;
        }
        if self.fullscreen {
            self.fullscreen = false;
        } else if self.focus != Tab::Files {
            self.focus = Tab::Files;
        } else {
            self.should_quit = true;
        }
    }

    fn click(&mut self, hit: Hit) {
        match hit {
            Hit::FileRow(index) => {
                self.selected = index;
                self.focus = Tab::Files;
                self.pane = Tab::Files;
                self.open_selected(false);
            }
            Hit::OutlineRow(index) => {
                self.outline_selected = index;
                self.jump_outline();
                self.focus = Tab::Outline;
                self.pane = Tab::Outline;
            }
            Hit::FocusTab(tab) => {
                self.focus = tab;
                if tab != Tab::Preview {
                    self.pane = tab;
                }
            }
            Hit::FilesPanel => {
                self.focus = Tab::Files;
                self.pane = Tab::Files;
            }
            Hit::OutlinePanel => {
                self.focus = Tab::Outline;
                self.pane = Tab::Outline;
            }
            Hit::PreviewPanel => self.focus = Tab::Preview,
            Hit::FilterRow => {
                self.fullscreen = false;
                self.pane = Tab::Files;
                self.focus = Tab::Files;
                self.filter_editing = true;
                self.filter.end();
            }
            Hit::SearchRow => {
                self.start_search();
            }
            Hit::CloseHelp => self.show_help = false,
        }
    }

    fn wheel(&mut self, mouse: &MouseEvent, delta: isize) {
        if inside(self.preview_rect, mouse) {
            self.focus = Tab::Preview;
            self.scroll_preview(delta * 3);
        } else if inside(self.files_rect, mouse) {
            self.focus = Tab::Files;
            self.move_selection(delta);
        } else if inside(self.outline_rect, mouse) {
            self.focus = Tab::Outline;
            self.move_outline(delta);
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            self.should_quit = true;
            return;
        }
        if self.show_help {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
            ) {
                self.show_help = false;
            }
            return;
        }
        if self.search_editing {
            match key.code {
                KeyCode::Esc => self.search_editing = false,
                KeyCode::Enter => self.confirm_search(),
                KeyCode::Backspace => self.search.backspace(),
                KeyCode::Delete => self.search.delete(),
                KeyCode::Left => self.search.left(),
                KeyCode::Right => self.search.right(),
                KeyCode::Home => self.search.home(),
                KeyCode::End => self.search.end(),
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.search = TextInput::new("");
                }
                KeyCode::Char(ch)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                        && !ch.is_control() =>
                {
                    self.search.insert(ch);
                }
                _ => {}
            }
            return;
        }
        if self.filter_editing {
            match key.code {
                KeyCode::Esc => self.filter_editing = false,
                KeyCode::Enter => {
                    self.filter_editing = false;
                    self.selected = 0;
                    self.open_selected(true);
                }
                KeyCode::Backspace => {
                    self.filter.backspace();
                    self.refresh_visible();
                }
                KeyCode::Delete => {
                    self.filter.delete();
                    self.refresh_visible();
                }
                KeyCode::Left => self.filter.left(),
                KeyCode::Right => self.filter.right(),
                KeyCode::Home => self.filter.home(),
                KeyCode::End => self.filter.end(),
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.filter = TextInput::new("");
                    self.refresh_visible();
                }
                KeyCode::Char(ch)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                        && !ch.is_control() =>
                {
                    self.filter.insert(ch);
                    self.refresh_visible();
                }
                _ => {}
            }
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('f') => self.start_search(),
                KeyCode::Char('d') => self.down(10),
                KeyCode::Char('u') => self.up(10),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => self.escape(),
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Tab => self.next_focus(),
            KeyCode::BackTab => self.prev_focus(),
            KeyCode::Char('/') => {
                self.fullscreen = false;
                self.pane = Tab::Files;
                self.focus = Tab::Files;
                self.filter_editing = true;
                self.filter.end();
            }
            KeyCode::Char('t') => self.cycle_theme(),
            KeyCode::Char('r') => self.rescan(),
            KeyCode::Char('m') => self.toggle_raw(),
            KeyCode::Char('n') => self.next_search(1),
            KeyCode::Char('N') => self.next_search(-1),
            KeyCode::Char('F') => self.toggle_fullscreen(),
            KeyCode::Char('e') => self.editor = self.doc.as_ref().map(|doc| doc.path.clone()),
            KeyCode::Char('s') => {
                self.sort = self.sort.next();
                self.refresh_visible();
                self.set_ok(format!("按{}排序啦喵~", self.sort.label()));
            }
            KeyCode::Down | KeyCode::Char('j') => self.down(1),
            KeyCode::Up | KeyCode::Char('k') => self.up(1),
            KeyCode::PageDown => self.down(self.page_step()),
            KeyCode::PageUp => self.up(self.page_step()),
            KeyCode::Home | KeyCode::Char('g') => self.top(),
            KeyCode::End | KeyCode::Char('G') => self.bottom(),
            KeyCode::Enter => self.activate(),
            _ => {}
        }
    }

    pub(crate) fn handle_paste(&mut self, text: &str) {
        if !self.filter_editing {
            return;
        }
        for ch in text.chars().filter(|ch| !ch.is_control()) {
            self.filter.insert(ch);
        }
        self.refresh_visible();
    }

    pub(crate) fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::Moved => {
                self.hover = self
                    .hits
                    .iter()
                    .rev()
                    .find(|(rect, _)| inside(*rect, &mouse))
                    .map(|(_, hit)| *hit);
            }
            MouseEventKind::ScrollDown => self.wheel(&mouse, 1),
            MouseEventKind::ScrollUp => self.wheel(&mouse, -1),
            MouseEventKind::Down(MouseButton::Left) => {
                let hit = self
                    .hits
                    .iter()
                    .rev()
                    .find(|(rect, _)| inside(*rect, &mouse))
                    .map(|(_, hit)| *hit);
                if let Some(hit) = hit {
                    self.click(hit);
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown" | "mdx"))
        .unwrap_or(false)
}

fn skip_dir(name: &str) -> bool {
    if name.starts_with('.') {
        return true;
    }
    matches!(
        name.to_ascii_lowercase().as_str(),
        "target"
            | "node_modules"
            | "dist"
            | "build"
            | "vendor"
            | "__pycache__"
            | "venv"
            | ".venv"
            | ".idea"
            | ".vscode"
    )
}

fn doc_file(path: &Path, rel: String) -> DocFile {
    let meta = fs::metadata(path).ok();
    DocFile {
        path: path.to_path_buf(),
        rel,
        size: meta.as_ref().map(|meta| meta.len()).unwrap_or(0),
        modified: meta.as_ref().and_then(|meta| meta.modified().ok()),
    }
}

pub(crate) fn scan_docs(root: &Path) -> Result<Vec<DocFile>, String> {
    if !root.exists() {
        return Err(format!("路径不存在喵~：{}", format::display_path(root)));
    }
    if root.is_file() {
        if !is_markdown(root) {
            return Err("这个文件不是 Markdown 喵~".to_string());
        }
        let rel = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "markdown".to_string());
        return Ok(vec![doc_file(root, rel)]);
    }
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if !skip_dir(&name) {
                    stack.push(path);
                }
            } else if kind.is_file() && is_markdown(&path) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(doc_file(&path, rel));
            }
        }
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(out)
}

pub(crate) fn parse_headings(content: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut fence: Option<char> = None;
    for (line, raw) in content.lines().enumerate() {
        let trimmed = raw.trim_start();
        let indent = raw.len().saturating_sub(trimmed.len());
        if indent > 3 {
            continue;
        }
        if let Some(first) = trimmed.chars().next()
            && (first == '`' || first == '~')
        {
            let run = trimmed.chars().take_while(|ch| *ch == first).count();
            if run >= 3 {
                match fence {
                    Some(mark) if mark == first => fence = None,
                    None => fence = Some(first),
                    _ => {}
                }
                continue;
            }
        }
        if fence.is_some() {
            continue;
        }
        let level = trimmed.chars().take_while(|ch| *ch == '#').count();
        if (1..=6).contains(&level) && trimmed.as_bytes().get(level) == Some(&b' ') {
            let title = crate::ui::strip_markdown_inline(trimmed[level + 1..].trim());
            if !title.is_empty() {
                out.push(Heading { level, title, line });
            }
        }
    }
    out
}

pub fn run(
    terminal: &mut ratatui::DefaultTerminal,
    root: &Path,
    theme: Option<String>,
) -> Result<()> {
    let paths = Paths::discover();
    let mut app = MdApp::new(paths, root.to_path_buf(), theme);
    loop {
        if let Some(path) = app.editor.take() {
            edit_file(terminal, &mut app, &path);
        }
        terminal.draw(|frame| crate::md_ui::draw(frame, &mut app))?;
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

fn edit_file(terminal: &mut ratatui::DefaultTerminal, app: &mut MdApp, path: &Path) {
    let editor = std::env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("EDITOR")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "vi".to_string());
    ratatui::restore();
    let _ = execute!(io::stdout(), DisableMouseCapture, DisableBracketedPaste);
    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("{editor} \"$1\""))
        .arg("gxt-docs")
        .arg(path)
        .status();
    let _ = execute!(io::stdout(), EnableMouseCapture, EnableBracketedPaste);
    *terminal = ratatui::init();
    match status {
        Ok(status) if status.success() => {
            let rel = app.rel_label();
            app.reload_current();
            app.set_ok(format!("编辑完啦喵~ 已重新读取 {rel}"));
        }
        Ok(status) => app.set_error(format!("编辑器退出码 {} 喵~", status.code().unwrap_or(-1))),
        Err(error) => app.set_error(format!("打不开编辑器喵~：{error}")),
    }
}

fn inside(rect: Rect, mouse: &MouseEvent) -> bool {
    rect.width > 0
        && rect.height > 0
        && mouse.column >= rect.x
        && mouse.column < rect.x + rect.width
        && mouse.row >= rect.y
        && mouse.row < rect.y + rect.height
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("gxt-docs-{name}-{}-{stamp}", std::process::id()))
    }

    #[test]
    fn parses_headings_and_skips_fenced_code() {
        let text = "# 标题\n\n```sh\n# 不是标题\n```\n## 二级 **粗体**\n    ### 缩进\n";
        let headings = parse_headings(text);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].title, "标题");
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[1].title, "二级 粗体");
    }

    #[test]
    fn scans_nested_markdown_and_skips_noise() {
        let root = temp_root("scan");
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join("target")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("README.md"), "# 根").unwrap();
        fs::write(root.join("docs/guide.markdown"), "# 指南").unwrap();
        fs::write(root.join("docs/notes.txt"), "no").unwrap();
        fs::write(root.join("target/skip.md"), "# skip").unwrap();
        fs::write(root.join(".git/skip.md"), "# skip").unwrap();
        let files = scan_docs(&root).unwrap();
        let rels: Vec<&str> = files.iter().map(|file| file.rel.as_str()).collect();
        assert_eq!(rels, vec!["README.md", "docs/guide.markdown"]);
        let _ = fs::remove_dir_all(&root);
    }
}
