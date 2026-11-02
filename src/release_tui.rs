use crate::app::TextInput;
use crate::config::{Config, Paths};
use crate::format;
use crate::icons;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use serde::Deserialize;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhRepo {
    name_with_owner: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    is_private: bool,
}

impl GhRepo {
    fn description(&self) -> &str {
        self.description.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Repos,
    Tag,
    Title,
    Target,
    Notes,
    Draft,
    Prerelease,
    Submit,
}

impl Field {
    fn next(self) -> Self {
        match self {
            Self::Repos => Self::Tag,
            Self::Tag => Self::Title,
            Self::Title => Self::Target,
            Self::Target => Self::Notes,
            Self::Notes => Self::Draft,
            Self::Draft => Self::Prerelease,
            Self::Prerelease => Self::Submit,
            Self::Submit => Self::Repos,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Repos => Self::Submit,
            Self::Tag => Self::Repos,
            Self::Title => Self::Tag,
            Self::Target => Self::Title,
            Self::Notes => Self::Target,
            Self::Draft => Self::Notes,
            Self::Prerelease => Self::Draft,
            Self::Submit => Self::Prerelease,
        }
    }
}

struct Form {
    tag: TextInput,
    title: TextInput,
    target: TextInput,
    notes: TextInput,
    draft: bool,
    prerelease: bool,
    field: Field,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            tag: TextInput::new(""),
            title: TextInput::new(""),
            target: TextInput::new(""),
            notes: TextInput::new(""),
            draft: false,
            prerelease: false,
            field: Field::Repos,
        }
    }
}

#[derive(Clone, Copy)]
enum StatusKind {
    Info,
    Ok,
    Error,
}

struct ReleaseApp {
    repos: Vec<GhRepo>,
    selected: usize,
    offset: usize,
    form: Form,
    theme: Theme,
    status: String,
    status_kind: StatusKind,
    publishing: bool,
    confirming: bool,
    should_quit: bool,
    started: Instant,
    tx: Sender<std::result::Result<String, String>>,
    rx: Receiver<std::result::Result<String, String>>,
}

pub fn run(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let paths = Paths::discover();
    let config = Config::load(&paths);
    let theme = Theme::load(&paths, &config.theme);
    let (tx, rx) = mpsc::channel();
    let mut app = ReleaseApp {
        repos: Vec::new(),
        selected: 0,
        offset: 0,
        form: Form::default(),
        theme,
        status: String::new(),
        status_kind: StatusKind::Info,
        publishing: false,
        confirming: false,
        should_quit: false,
        started: Instant::now(),
        tx,
        rx,
    };
    app.reload_repos();
    loop {
        app.drain();
        terminal.draw(|frame| draw(frame, &mut app))?;
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

fn load_repos() -> std::result::Result<Vec<GhRepo>, String> {
    let output = Command::new("gh")
        .args([
            "repo",
            "list",
            "--limit",
            "200",
            "--json",
            "nameWithOwner,description,isPrivate",
        ])
        .output()
        .map_err(|err| format!("跑不了 gh 喵~：{err}"))?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if error.is_empty() {
            "gh repo list 失败啦喵~".to_string()
        } else {
            error
        });
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("gh 返回的仓库列表看不懂喵~：{err}"))
}

fn run_gh(args: &[String]) -> std::result::Result<String, String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .map_err(|err| format!("跑不了 gh 喵~：{err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if error.is_empty() { stdout } else { error })
    }
}

fn release_args(repo: &str, form: &Form) -> Vec<String> {
    let mut args = vec![
        "release".to_string(),
        "create".to_string(),
        form.tag.value.trim().to_string(),
        "--repo".to_string(),
        repo.to_string(),
    ];
    let title = form.title.value.trim();
    if !title.is_empty() {
        args.push("--title".to_string());
        args.push(title.to_string());
    }
    let notes = form.notes.value.trim();
    if notes.is_empty() {
        args.push("--generate-notes".to_string());
    } else {
        args.push("--notes".to_string());
        args.push(notes.to_string());
    }
    let target = form.target.value.trim();
    if !target.is_empty() {
        args.push("--target".to_string());
        args.push(target.to_string());
    }
    if form.draft {
        args.push("--draft".to_string());
    }
    if form.prerelease {
        args.push("--prerelease".to_string());
    }
    args
}

fn draw(frame: &mut Frame, app: &mut ReleaseApp) {
    let area = frame.area();
    let theme = &app.theme;
    frame.render_widget(
        Block::new().style(Style::default().bg(theme.color("base"))),
        area,
    );
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .split(area);
    let header = Line::from(vec![
        Span::styled(
            format!("{} ", icons::TAG),
            Style::default().fg(theme.color("peach")),
        ),
        Span::styled(
            "个人发行版发布喵~",
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "   用 gh 选仓库、填表单、发布喵~",
            Style::default().fg(theme.color("overlay0")),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(header),
        Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1),
    );
    let columns = Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(chunks[1]);
    draw_repos(frame, app, columns[0]);
    draw_form(frame, app, columns[1]);
    draw_footer(frame, app, chunks[2]);
    if app.confirming {
        draw_confirm(frame, app, area);
    }
}

fn draw_confirm(frame: &mut Frame, app: &ReleaseApp, area: Rect) {
    let theme = &app.theme;
    let width = area.width.saturating_sub(12).min(64);
    let height = 7;
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("peach"))),
        rect,
    );
    let repo = app.selected_repo().unwrap_or("?");
    let tag = app.form.tag.value.trim();
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "确认发布喵~",
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!(
                "{} → {}",
                format::truncate(repo, 30),
                format::truncate(tag, 20)
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Enter 确认发布    Esc 取消喵~",
                Style::default().fg(theme.color("overlay0")),
            )),
        ])
        .alignment(Alignment::Center),
        rect.inner(ratatui::layout::Margin::new(2, 1)),
    );
}

fn draw_repos(frame: &mut Frame, app: &mut ReleaseApp, area: Rect) {
    let theme = &app.theme;
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.color("surface1")))
        .title(icons::label(icons::REPO, "个人仓库喵~"));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if app.repos.is_empty() {
        frame.render_widget(
            Paragraph::new(format::truncate(&app.status, inner.width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            inner,
        );
        return;
    }
    let rows = inner.height.max(1) as usize;
    if app.selected < app.offset {
        app.offset = app.selected;
    } else if app.selected >= app.offset + rows {
        app.offset = app.selected + 1 - rows;
    }
    for (row, index) in (app.offset..app.repos.len()).take(rows).enumerate() {
        let repo = &app.repos[index];
        let rect = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);
        let selected = index == app.selected;
        if selected {
            frame.render_widget(
                Block::new().style(Style::default().bg(theme.color("surface0"))),
                rect,
            );
        }
        let icon = if repo.is_private {
            icons::LOCK
        } else {
            icons::GLOBE
        };
        let name_width = inner.width.saturating_sub(3) as usize;
        let name = format::truncate(&repo.name_with_owner, name_width);
        let mut spans = vec![
            Span::styled(
                format!("{icon} "),
                Style::default().fg(if repo.is_private {
                    theme.color("peach")
                } else {
                    theme.color("overlay1")
                }),
            ),
            Span::styled(
                name,
                Style::default()
                    .fg(if selected {
                        theme.color("text")
                    } else {
                        theme.color("subtext1")
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        if !repo.description().trim().is_empty() && inner.width >= 40 {
            spans.push(Span::styled(
                format!("  {}", format::clean_text(repo.description())),
                Style::default().fg(theme.color("overlay0")),
            ));
        }
        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(rect.x + 1, rect.y, rect.width.saturating_sub(1), 1),
        );
    }
}

fn draw_form(frame: &mut Frame, app: &mut ReleaseApp, area: Rect) {
    let theme = &app.theme;
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.color("surface1")))
        .title(icons::label(icons::TAG, "发布表单喵~"));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height < 12 || inner.width < 20 {
        return;
    }
    let mut y = inner.y;
    draw_field(
        frame,
        app,
        Rect::new(inner.x, y, inner.width, 1),
        "标签喵~",
        Field::Tag,
    );
    y += 1;
    draw_field(
        frame,
        app,
        Rect::new(inner.x, y, inner.width, 1),
        "标题喵~",
        Field::Title,
    );
    y += 1;
    draw_field(
        frame,
        app,
        Rect::new(inner.x, y, inner.width, 1),
        "目标分支喵~",
        Field::Target,
    );
    y += 2;
    let notes_height = inner.height.saturating_sub((y - inner.y) + 6).clamp(3, 12);
    let notes_rect = Rect::new(inner.x + 1, y, inner.width.saturating_sub(2), notes_height);
    draw_notes(frame, app, notes_rect);
    y += notes_height + 1;
    draw_toggle(
        frame,
        app,
        Rect::new(inner.x, y, inner.width, 1),
        "草稿",
        Field::Draft,
        app.form.draft,
    );
    y += 1;
    draw_toggle(
        frame,
        app,
        Rect::new(inner.x, y, inner.width, 1),
        "预发布",
        Field::Prerelease,
        app.form.prerelease,
    );
    y += 2;
    let publish = if app.publishing {
        format!(
            "{} 发布中喵~",
            format::spinner(app.started.elapsed().as_millis() as u64 / 120)
        )
    } else if app.form.field == Field::Submit {
        icons::label(icons::TAG, "按 Enter 发布喵~")
    } else {
        icons::label(icons::TAG, "发布喵~")
    };
    let publish_width = format::text_width(&publish) as u16 + 4;
    let focused = app.form.field == Field::Submit;
    frame.render_widget(
        Block::new().style(Style::default().bg(if focused {
            theme.color("blue")
        } else {
            theme.color("surface0")
        })),
        Rect::new(inner.x, y, publish_width.min(inner.width), 1),
    );
    frame.render_widget(
        Paragraph::new(publish)
            .alignment(Alignment::Center)
            .style(Style::default().fg(if focused {
                theme.color("base")
            } else {
                theme.color("text")
            })),
        Rect::new(inner.x, y, publish_width.min(inner.width), 1),
    );
}

fn draw_field(frame: &mut Frame, app: &ReleaseApp, rect: Rect, label: &str, field: Field) {
    let theme = &app.theme;
    let focused = app.form.field == field;
    if focused {
        frame.render_widget(
            Block::new().style(Style::default().bg(theme.color("surface0"))),
            rect,
        );
    }
    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(if focused {
            theme.color("blue")
        } else {
            theme.color("overlay0")
        })),
        Rect::new(rect.x + 1, rect.y, 12, 1),
    );
    let input = match field {
        Field::Tag => &app.form.tag,
        Field::Title => &app.form.title,
        Field::Target => &app.form.target,
        _ => return,
    };
    let value_width = rect.width.saturating_sub(14);
    let (visible, cursor) = if input.is_empty() {
        let placeholder = match field {
            Field::Tag => "v1.2.3（必填）",
            Field::Title => "默认和标签一样",
            _ => "默认分支",
        };
        (format::truncate(placeholder, value_width as usize), 0)
    } else {
        format::visible_text(&input.value, input.cursor, value_width as usize)
    };
    frame.render_widget(
        Paragraph::new(visible).style(Style::default().fg(if input.is_empty() {
            theme.color("overlay0")
        } else {
            theme.color("text")
        })),
        Rect::new(rect.x + 13, rect.y, value_width, 1),
    );
    if focused {
        frame.set_cursor_position(ratatui::layout::Position::new(
            (rect.x + 13 + cursor).min(rect.x + 12 + value_width),
            rect.y,
        ));
    }
}

fn draw_notes(frame: &mut Frame, app: &ReleaseApp, area: Rect) {
    let theme = &app.theme;
    let focused = app.form.field == Field::Notes;
    frame.render_widget(
        Paragraph::new("发行说明喵~（留空自动生成）").style(Style::default().fg(if focused {
            theme.color("blue")
        } else {
            theme.color("overlay0")
        })),
        Rect::new(area.x, area.y.saturating_sub(1), area.width, 1),
    );
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused {
            theme.color("blue")
        } else {
            theme.color("surface1")
        }));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let width = inner.width.max(1) as usize;
    let mut lines: Vec<Line> = Vec::new();
    for logical in app.form.notes.value.split('\n') {
        if logical.is_empty() {
            lines.push(Line::default());
            continue;
        }
        for chunk in format::wrap_text(logical, width, 8) {
            lines.push(Line::from(Span::styled(
                chunk,
                Style::default().fg(theme.color("text")),
            )));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "写点更新内容喵~",
            Style::default().fg(theme.color("overlay0")),
        )));
    }
    frame.render_widget(Paragraph::new(lines), inner);
    if focused {
        let last = app.form.notes.value.rsplit('\n').next().unwrap_or("");
        let (visible, cursor) = format::visible_text(last, last.len(), width);
        let rows = format::wrap_text(last, width, 8).len().max(1);
        let _ = visible;
        let column = cursor.min(width.saturating_sub(1) as u16);
        let row = (rows.saturating_sub(1)).min(inner.height.saturating_sub(1) as usize) as u16;
        frame.set_cursor_position(ratatui::layout::Position::new(
            inner.x + column,
            inner.y + row,
        ));
    }
}

fn draw_toggle(
    frame: &mut Frame,
    app: &ReleaseApp,
    rect: Rect,
    label: &str,
    field: Field,
    enabled: bool,
) {
    let theme = &app.theme;
    let focused = app.form.field == field;
    if focused {
        frame.render_widget(
            Block::new().style(Style::default().bg(theme.color("surface0"))),
            rect,
        );
    }
    let icon = if enabled {
        icons::CHECK_SQUARE
    } else {
        icons::SQUARE
    };
    frame.render_widget(
        Paragraph::new(format!("{icon} {label}喵~")).style(Style::default().fg(if enabled {
            theme.color("green")
        } else if focused {
            theme.color("text")
        } else {
            theme.color("subtext1")
        })),
        Rect::new(rect.x + 1, rect.y, rect.width.saturating_sub(2), 1),
    );
}

fn draw_footer(frame: &mut Frame, app: &ReleaseApp, area: Rect) {
    let theme = &app.theme;
    let (icon, color) = match app.status_kind {
        StatusKind::Info => (icons::INFO, theme.color("subtext0")),
        StatusKind::Ok => (icons::CHECK, theme.color("green")),
        StatusKind::Error => (icons::WARNING, theme.color("red")),
    };
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("{icon}  {}", app.status),
            area.width.saturating_sub(2) as usize,
        ))
        .style(Style::default().fg(color)),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    let hint = if app.publishing {
        "发布中，等 gh 返回喵~"
    } else if app.confirming {
        "Enter 确认发布  Esc 取消喵~"
    } else {
        "j/k 选仓库  Tab 换字段  Ctrl+S 发布  Esc 退出喵~"
    };
    frame.render_widget(
        Paragraph::new(format::truncate(
            hint,
            area.width.saturating_sub(2) as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 1),
    );
}

impl ReleaseApp {
    fn reload_repos(&mut self) {
        match load_repos() {
            Ok(repos) => {
                let count = repos.len();
                self.repos = repos;
                self.selected = 0;
                self.offset = 0;
                self.set_status(StatusKind::Info, &format!("从 gh 读到 {count} 个仓库喵~"));
            }
            Err(error) => {
                self.repos.clear();
                self.set_status(StatusKind::Error, &error);
            }
        }
    }

    fn set_status(&mut self, kind: StatusKind, text: &str) {
        self.status_kind = kind;
        self.status = text.to_string();
    }

    fn drain(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            self.publishing = false;
            match result {
                Ok(output) => {
                    self.form.tag = TextInput::new("");
                    self.form.title = TextInput::new("");
                    self.form.notes = TextInput::new("");
                    self.form.field = Field::Tag;
                    let text = if output.is_empty() {
                        "发行版发布好啦喵~".to_string()
                    } else {
                        format!("发行版发布好啦喵~ {output}")
                    };
                    self.set_status(StatusKind::Ok, &text);
                }
                Err(error) => self.set_status(StatusKind::Error, &error),
            }
        }
    }

    fn selected_repo(&self) -> Option<&str> {
        self.repos
            .get(self.selected)
            .map(|repo| repo.name_with_owner.as_str())
    }

    fn text_input_mut(&mut self) -> Option<&mut TextInput> {
        match self.form.field {
            Field::Tag => Some(&mut self.form.tag),
            Field::Title => Some(&mut self.form.title),
            Field::Target => Some(&mut self.form.target),
            Field::Notes => Some(&mut self.form.notes),
            _ => None,
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if self.publishing {
            return;
        }
        let multiline = self.form.field == Field::Notes;
        if let Some(input) = self.text_input_mut() {
            for ch in text.chars() {
                if ch == '\r' {
                    continue;
                }
                if ch == '\n' && !multiline {
                    continue;
                }
                if !ch.is_control() {
                    input.insert(ch);
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.confirming {
            match key.code {
                KeyCode::Enter => {
                    self.confirming = false;
                    self.publish();
                }
                KeyCode::Esc => {
                    self.confirming = false;
                    self.set_status(StatusKind::Info, "取消发布啦喵~");
                }
                _ => {}
            }
            return;
        }
        if self.publishing {
            return;
        }
        if key.code == KeyCode::Esc {
            self.should_quit = true;
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('s') | KeyCode::Char('S'))
        {
            self.submit();
            return;
        }
        match key.code {
            KeyCode::Tab => {
                self.form.field = self.form.field.next();
                return;
            }
            KeyCode::BackTab => {
                self.form.field = self.form.field.prev();
                return;
            }
            _ => {}
        }
        match self.form.field {
            Field::Repos => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if !self.repos.is_empty() {
                        self.selected = (self.selected + 1).min(self.repos.len() - 1);
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.selected = self.selected.saturating_sub(1);
                }
                KeyCode::Char('g') | KeyCode::Home => self.selected = 0,
                KeyCode::Char('G') | KeyCode::End => {
                    self.selected = self.repos.len().saturating_sub(1);
                }
                KeyCode::Char('r') => self.reload_repos(),
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Enter => self.form.field = Field::Tag,
                _ => {}
            },
            Field::Draft => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => self.form.draft = !self.form.draft,
                _ => {}
            },
            Field::Prerelease => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.form.prerelease = !self.form.prerelease;
                }
                _ => {}
            },
            Field::Submit => {
                if key.code == KeyCode::Enter {
                    self.submit();
                }
            }
            _ => {
                if key.code == KeyCode::Enter && self.form.field != Field::Notes {
                    self.form.field = self.form.field.next();
                    return;
                }
                let multiline = self.form.field == Field::Notes;
                let Some(input) = self.text_input_mut() else {
                    return;
                };
                match key.code {
                    KeyCode::Enter => input.insert('\n'),
                    KeyCode::Backspace => input.backspace(),
                    KeyCode::Delete => input.delete(),
                    KeyCode::Left => input.left(),
                    KeyCode::Right => input.right(),
                    KeyCode::Home => input.home(),
                    KeyCode::End => input.end(),
                    KeyCode::Char(ch) if !ch.is_control() && (multiline || ch != '\n') => {
                        input.insert(ch);
                    }
                    _ => {}
                }
            }
        }
    }

    fn submit(&mut self) {
        if self.publishing {
            return;
        }
        let Some(repo) = self.selected_repo().map(str::to_string) else {
            self.set_status(StatusKind::Error, "先在左边选一个仓库喵~");
            return;
        };
        let tag = self.form.tag.value.trim().to_string();
        if tag.is_empty() {
            self.form.field = Field::Tag;
            self.set_status(StatusKind::Error, "标签不能空着喵~");
            return;
        }
        if tag.chars().any(char::is_whitespace) {
            self.form.field = Field::Tag;
            self.set_status(StatusKind::Error, "标签里不能有空格喵~");
            return;
        }
        self.confirming = true;
        self.set_status(
            StatusKind::Info,
            &format!("要发布 {repo} {tag} 吗？Enter 确认，Esc 取消喵~"),
        );
    }

    fn publish(&mut self) {
        if self.publishing {
            return;
        }
        let Some(repo) = self.selected_repo().map(str::to_string) else {
            return;
        };
        let tag = self.form.tag.value.trim().to_string();
        if tag.is_empty() {
            self.form.field = Field::Tag;
            self.set_status(StatusKind::Error, "标签不能空着喵~");
            return;
        }
        let args = release_args(&repo, &self.form);
        let tx = self.tx.clone();
        self.publishing = true;
        self.set_status(StatusKind::Info, &format!("正在给 {repo} 发布 {tag} 喵~"));
        std::thread::spawn(move || {
            let _ = tx.send(run_gh(&args));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_release_args() {
        let form = Form {
            tag: TextInput::new("v0.2.2"),
            title: TextInput::new("Release 0.2.2"),
            target: TextInput::new("main"),
            draft: true,
            ..Form::default()
        };
        let args = release_args("owner/repo", &form);
        assert_eq!(args[0], "release");
        assert_eq!(args[1], "create");
        assert_eq!(args[2], "v0.2.2");
        assert!(args.contains(&"--repo".to_string()));
        assert!(args.contains(&"--title".to_string()));
        assert!(args.contains(&"--generate-notes".to_string()));
        assert!(args.contains(&"--draft".to_string()));
        let form = Form {
            notes: TextInput::new("hello"),
            ..form
        };
        let args = release_args("owner/repo", &form);
        assert!(args.contains(&"--notes".to_string()));
        assert!(args.contains(&"hello".to_string()));
        assert!(!args.contains(&"--generate-notes".to_string()));
    }

    #[test]
    fn parses_gh_repo_json() {
        let repos: Vec<GhRepo> = serde_json::from_str(
            r#"[{"nameWithOwner":"a/b","description":"demo","isPrivate":true}]"#,
        )
        .unwrap();
        assert_eq!(repos[0].name_with_owner, "a/b");
        assert!(repos[0].is_private);
        assert_eq!(Field::Repos.next(), Field::Tag);
        assert_eq!(Field::Repos.prev(), Field::Submit);
    }
}
