use crate::app::TextInput;
use crate::config::{Config, Paths};
use crate::format;
use crate::icons;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use serde::Deserialize;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

const CARD_HEIGHT: u16 = 4;
const CARD_STEP: u16 = 5;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhRepo {
    name_with_owner: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    is_private: bool,
    #[serde(default)]
    stargazer_count: u64,
    #[serde(default)]
    fork_count: u64,
    #[serde(default)]
    updated_at: String,
}

impl GhRepo {
    fn description(&self) -> &str {
        self.description.as_deref().unwrap_or("")
    }

    fn owner(&self) -> &str {
        self.name_with_owner
            .split_once('/')
            .map(|(owner, _)| owner)
            .unwrap_or("")
    }

    fn name(&self) -> &str {
        self.name_with_owner
            .split_once('/')
            .map(|(_, name)| name)
            .unwrap_or(&self.name_with_owner)
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
    GenerateNotes,
    Latest,
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
            Self::Prerelease => Self::GenerateNotes,
            Self::GenerateNotes => Self::Latest,
            Self::Latest => Self::Submit,
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
            Self::GenerateNotes => Self::Prerelease,
            Self::Latest => Self::GenerateNotes,
            Self::Submit => Self::Latest,
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
    generate_notes: bool,
    latest: bool,
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
            generate_notes: true,
            latest: false,
            field: Field::Repos,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
enum Action {
    SelectRepo(usize),
    Focus(Field),
    Toggle(Field),
    Publish,
    Clear,
    Confirm,
    Cancel,
    CloseHelp,
}

struct HitBox {
    rect: Rect,
    action: Action,
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
    repos_loading: bool,
    confirming: bool,
    show_help: bool,
    should_quit: bool,
    started: Instant,
    hits: Vec<HitBox>,
    hover: Option<Action>,
    list_rect: Rect,
    overlay_rect: Rect,
    tx: Sender<std::result::Result<String, String>>,
    rx: Receiver<std::result::Result<String, String>>,
    repo_tx: Sender<std::result::Result<Vec<GhRepo>, String>>,
    repo_rx: Receiver<std::result::Result<Vec<GhRepo>, String>>,
}

pub fn run(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let paths = Paths::discover();
    let config = Config::load(&paths);
    let (tx, rx) = mpsc::channel();
    let (repo_tx, repo_rx) = mpsc::channel();
    let mut app = ReleaseApp {
        repos: Vec::new(),
        selected: 0,
        offset: 0,
        form: Form::default(),
        theme: Theme::load(&paths, &config.theme),
        status: String::new(),
        status_kind: StatusKind::Info,
        publishing: false,
        repos_loading: false,
        confirming: false,
        show_help: false,
        should_quit: false,
        started: Instant::now(),
        hits: Vec::new(),
        hover: None,
        list_rect: Rect::default(),
        overlay_rect: Rect::default(),
        tx,
        rx,
        repo_tx,
        repo_rx,
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

fn register(hits: &mut Vec<HitBox>, rect: Rect, action: Action) {
    hits.push(HitBox { rect, action });
}

fn fill(frame: &mut Frame, rect: Rect, color: ratatui::style::Color) {
    frame.render_widget(Block::new().style(Style::default().bg(color)), rect);
}

fn load_repos() -> std::result::Result<Vec<GhRepo>, String> {
    let output = Command::new("gh")
        .args([
            "repo",
            "list",
            "--limit",
            "200",
            "--json",
            "nameWithOwner,description,isPrivate,stargazerCount,forkCount,updatedAt",
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
    if !notes.is_empty() {
        args.push("--notes".to_string());
        args.push(notes.to_string());
    } else if form.generate_notes {
        args.push("--generate-notes".to_string());
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
    if form.latest {
        args.push("--latest".to_string());
    }
    args
}

fn command_preview(repo: &str, form: &Form) -> String {
    let args = release_args(repo, form);
    let rendered: Vec<String> = args
        .iter()
        .map(|arg| {
            if arg.chars().any(char::is_whitespace) {
                format!("\"{}\"", arg.replace('"', "\\\""))
            } else {
                arg.clone()
            }
        })
        .collect();
    format!("gh {}", rendered.join(" "))
}

fn draw(frame: &mut Frame, app: &mut ReleaseApp) {
    let area = frame.area();
    let theme = app.theme.clone();
    app.hits.clear();
    app.overlay_rect = Rect::default();
    frame.render_widget(
        Block::new().style(Style::default().bg(theme.color("base"))),
        area,
    );
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .split(area);
    draw_header(frame, app, chunks[0]);
    let inner = Rect::new(
        chunks[1].x + 2,
        chunks[1].y + 1,
        chunks[1].width.saturating_sub(4),
        chunks[1].height.saturating_sub(2),
    );
    let columns = Layout::horizontal([
        Constraint::Percentage(39),
        Constraint::Length(1),
        Constraint::Percentage(60),
    ])
    .split(inner);
    draw_repos(frame, app, columns[0]);
    draw_form(frame, app, columns[2]);
    draw_footer(frame, app, chunks[2]);
    if app.show_help {
        app.hits.clear();
        draw_help(frame, app, area);
    } else if app.confirming {
        app.hits.clear();
        draw_confirm(frame, app, area);
    }
}

fn draw_header(frame: &mut Frame, app: &ReleaseApp, area: Rect) {
    let theme = &app.theme;
    frame.render_widget(
        Block::new().style(Style::default().bg(theme.color("mantle"))),
        area,
    );
    let title = Line::from(vec![
        Span::styled(
            format!("{} ", icons::GITHUB),
            Style::default().fg(theme.color("blue")),
        ),
        Span::styled(
            "gxt",
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  个人发行版发布喵~",
            Style::default().fg(theme.color("subtext0")),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(title),
        Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1),
    );
    if area.width > 100 {
        let account = format!(
            "{} gh 仓库 {} 个喵~   {} 发布表单喵~",
            icons::REPO,
            if app.repos_loading {
                "读取中".to_string()
            } else {
                app.repos.len().to_string()
            },
            icons::TAG
        );
        frame.render_widget(
            Paragraph::new(account)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1),
        );
    }
    let tab = icons::label(icons::TAG, "个人发行版喵~");
    let tab_width = format::text_width(&tab) as u16 + 4;
    let tab_rect = Rect::new(area.x + 2, area.y + 1, tab_width, 1);
    frame.render_widget(
        Paragraph::new(tab).alignment(Alignment::Center).style(
            Style::default()
                .fg(theme.color("blue"))
                .bg(theme.color("surface1"))
                .add_modifier(Modifier::BOLD),
        ),
        tab_rect,
    );
    frame.render_widget(
        Paragraph::new(" gh release create ").style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            area.x + tab_width + 2,
            area.y + 1,
            area.width.saturating_sub(tab_width + 4),
            1,
        ),
    );
    if area.width > 100 {
        let hint = "Tab 换字段   Ctrl+S 发布   ? 帮助   Esc 退出喵~";
        frame.render_widget(
            Paragraph::new(hint)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
        );
    }
    frame.render_widget(
        Block::new().style(Style::default().bg(theme.color("surface0"))),
        Rect::new(area.x, area.y + 2, area.width, 1),
    );
}

fn draw_footer(frame: &mut Frame, app: &ReleaseApp, area: Rect) {
    let theme = &app.theme;
    frame.render_widget(
        Block::new().style(Style::default().bg(theme.color("crust"))),
        area,
    );
    let (icon, color) = match app.status_kind {
        StatusKind::Info => (icons::INFO, theme.color("subtext0")),
        StatusKind::Ok => (icons::CHECK, theme.color("green")),
        StatusKind::Error => (icons::WARNING, theme.color("red")),
    };
    let busy = app.publishing || app.repos_loading;
    let status = if busy {
        format!(
            "{}  {}",
            format::spinner(app.started.elapsed().as_millis() as u64 / 120),
            app.status
        )
    } else {
        format!("{icon}  {}", app.status)
    };
    let status_color = if busy { theme.color("peach") } else { color };
    frame.render_widget(
        Paragraph::new(format::truncate(
            &status,
            area.width.saturating_sub(4) as usize,
        ))
        .style(Style::default().fg(status_color)),
        Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1),
    );
    let hints = if app.confirming {
        "Enter 确认发布   Esc 取消喵~"
    } else if app.show_help {
        "Esc 关掉帮助喵~"
    } else if app.publishing {
        "发布中，等 gh 返回喵~ Esc 暂时不能退出喵~"
    } else {
        "j/k 选仓库   Tab 换字段   Ctrl+S 发布   r 刷新仓库   Esc 退出喵~"
    };
    let right = format!("gxt {} · gh", env!("CARGO_PKG_VERSION"));
    let right_width = format::text_width(&right) as u16;
    let left_width = area
        .width
        .saturating_sub(right_width + 5)
        .min(format::text_width(hints) as u16);
    frame.render_widget(
        Paragraph::new(format::truncate(hints, left_width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 2, area.y + 1, left_width, 1),
    );
    frame.render_widget(
        Paragraph::new(right)
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
    );
}

fn draw_repos(frame: &mut Frame, app: &mut ReleaseApp, area: Rect) {
    let theme = &app.theme;
    app.list_rect = area;
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Paragraph::new(icons::label(icons::REPO, "个人仓库喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    frame.render_widget(
        Paragraph::new(format!("{} 个喵~", app.repos.len()))
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(3), 1),
    );
    frame.render_widget(
        Block::new().style(Style::default().bg(theme.color("surface0"))),
        Rect::new(area.x, area.y + 1, area.width, 1),
    );
    let body = Rect::new(
        area.x + 1,
        area.y + 2,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    if body.height < CARD_HEIGHT {
        return;
    }
    if app.repos.is_empty() {
        let (title, detail) = if app.repos_loading {
            (
                format!(
                    "{} 正在用 gh 读仓库喵~...",
                    format::spinner(app.started.elapsed().as_millis() as u64 / 120)
                ),
                "界面已经打开啦，不用等它读完喵~".to_string(),
            )
        } else {
            (
                "还没有读到仓库喵~".to_string(),
                "先运行 gh auth login，再按 r 刷新喵~".to_string(),
            )
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    title,
                    Style::default().fg(theme.color("text")),
                )),
                Line::default(),
                Line::from(Span::styled(
                    detail,
                    Style::default().fg(theme.color("overlay0")),
                )),
            ])
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(body.x, body.y + body.height / 3, body.width, 3),
        );
        return;
    }
    let viewport = (body.height / CARD_STEP).max(1) as usize;
    if app.selected < app.offset {
        app.offset = app.selected;
    } else if app.selected >= app.offset + viewport {
        app.offset = app.selected + 1 - viewport;
    }
    let max_offset = app.repos.len().saturating_sub(viewport);
    app.offset = app.offset.min(max_offset);
    for (row, index) in (app.offset..app.repos.len()).take(viewport).enumerate() {
        let card = Rect::new(
            body.x,
            body.y + row as u16 * CARD_STEP,
            body.width.saturating_sub(1),
            CARD_HEIGHT,
        );
        let action = Action::SelectRepo(index);
        let hovered = app.hover.as_ref() == Some(&action);
        draw_repo_card(frame, app, card, index, hovered);
        register(&mut app.hits, card, action);
    }
    if app.repos.len() > viewport {
        let track = Rect::new(
            body.x + body.width.saturating_sub(1),
            body.y,
            1,
            body.height,
        );
        fill(frame, track, theme.color("surface0"));
        let thumb_height =
            (body.height as f32 * viewport as f32 / app.repos.len() as f32).max(1.0) as u16;
        let span = body.height.saturating_sub(thumb_height).max(1);
        let offset = (app.offset as f32 / max_offset.max(1) as f32 * span as f32) as u16;
        fill(
            frame,
            Rect::new(track.x, track.y + offset, 1, thumb_height),
            theme.color("overlay0"),
        );
    }
}

fn draw_repo_card(frame: &mut Frame, app: &ReleaseApp, area: Rect, index: usize, hovered: bool) {
    let theme = &app.theme;
    let repo = &app.repos[index];
    let selected = index == app.selected;
    let bg = if selected || hovered {
        theme.color("surface0")
    } else {
        theme.color("mantle")
    };
    fill(frame, area, bg);
    if selected {
        fill(
            frame,
            Rect::new(area.x, area.y, 1, area.height),
            theme.color("blue"),
        );
    } else if hovered {
        fill(
            frame,
            Rect::new(area.x, area.y, 1, area.height),
            theme.color("surface2"),
        );
    }
    let left = area.x + 2;
    let width = area.width.saturating_sub(4);
    let visibility = if repo.is_private {
        format!("{} 私有喵~", icons::LOCK)
    } else {
        format!("{} 公开喵~", icons::GLOBE)
    };
    let right_width = format::text_width(&visibility) as u16 + 1;
    let title = Line::from(vec![
        Span::styled(
            format!("{} ", icons::REPO),
            Style::default().fg(theme.color("blue")),
        ),
        Span::styled(
            repo.owner().to_string(),
            Style::default().fg(theme.color("overlay1")),
        ),
        Span::styled("/", Style::default().fg(theme.color("overlay0"))),
        Span::styled(
            repo.name().to_string(),
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(title),
        Rect::new(left, area.y, width.saturating_sub(right_width), 1),
    );
    frame.render_widget(
        Paragraph::new(visibility)
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(left, area.y, width, 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format::clean_text(repo.description()),
            width as usize,
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(left, area.y + 1, width, 1),
    );
    let meta = format!(
        "{} {}   {} {}   {} {}",
        icons::STAR,
        format::human_count(repo.stargazer_count),
        icons::FORK,
        format::human_count(repo.fork_count),
        icons::CLOCK,
        format::relative_time(&repo.updated_at)
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&meta, width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(left, area.y + 2, width, 1),
    );
}

fn draw_form(frame: &mut Frame, app: &mut ReleaseApp, area: Rect) {
    let theme = &app.theme;
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Paragraph::new(icons::label(icons::TAG, "发布表单喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    let repo_label = app
        .repos
        .get(app.selected)
        .map(|repo| repo.name_with_owner.clone())
        .unwrap_or_else(|| "还没选仓库喵~".to_string());
    frame.render_widget(
        Paragraph::new(format::truncate(
            &repo_label,
            area.width.saturating_sub(4) as usize,
        ))
        .alignment(Alignment::Right)
        .style(Style::default().fg(theme.color("blue"))),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(3), 1),
    );
    frame.render_widget(
        Paragraph::new("填标签和说明，Ctrl+S 或点发布按钮喵~ 说明留空会按开关自动生成喵~")
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 1),
    );
    frame.render_widget(
        Block::new().style(Style::default().bg(theme.color("surface0"))),
        Rect::new(area.x, area.y + 2, area.width, 1),
    );
    let content = Rect::new(
        area.x + 1,
        area.y + 3,
        area.width.saturating_sub(2),
        area.height.saturating_sub(3),
    );
    if content.height < 16 || content.width < 24 {
        return;
    }
    let mut y = content.y;
    draw_field(
        frame,
        app,
        Rect::new(content.x, y, content.width, 1),
        Field::Tag,
        icons::TAG,
        "标签喵~",
        "v0.2.2（必填）",
    );
    y += 1;
    draw_field(
        frame,
        app,
        Rect::new(content.x, y, content.width, 1),
        Field::Title,
        icons::INFO,
        "标题喵~",
        "默认和标签一样喵~",
    );
    y += 1;
    draw_field(
        frame,
        app,
        Rect::new(content.x, y, content.width, 1),
        Field::Target,
        icons::BRANCH,
        "目标分支喵~",
        "默认分支喵~",
    );
    y += 2;
    let reserved = 2 + 2 + 5 + 2;
    let notes_height = content
        .bottom()
        .saturating_sub(y)
        .saturating_sub(reserved)
        .clamp(3, 10);
    draw_notes(
        frame,
        app,
        Rect::new(content.x, y, content.width, notes_height + 1),
    );
    y += notes_height + 2;
    draw_toggles(frame, app, Rect::new(content.x, y, content.width, 2));
    y += 3;
    draw_preview(frame, app, Rect::new(content.x, y, content.width, 5));
    let buttons_y = content.bottom().saturating_sub(1);
    draw_buttons(
        frame,
        app,
        Rect::new(content.x, buttons_y, content.width, 1),
    );
}

fn field_focused(app: &ReleaseApp, field: Field) -> bool {
    app.form.field == field
}

fn draw_field(
    frame: &mut Frame,
    app: &mut ReleaseApp,
    rect: Rect,
    field: Field,
    icon: &str,
    label: &str,
    placeholder: &str,
) {
    let theme = &app.theme;
    let focused = field_focused(app, field);
    let hovered = app.hover.as_ref() == Some(&Action::Focus(field));
    let input = match field {
        Field::Tag => &app.form.tag,
        Field::Title => &app.form.title,
        Field::Target => &app.form.target,
        _ => return,
    };
    if focused || hovered {
        fill(
            frame,
            rect,
            if focused {
                theme.color("surface0")
            } else {
                theme.color("surface1")
            },
        );
    }
    frame.render_widget(
        Paragraph::new(format!("{icon} {label}")).style(Style::default().fg(if focused {
            theme.color("blue")
        } else if hovered {
            theme.color("text")
        } else {
            theme.color("overlay0")
        })),
        Rect::new(rect.x + 1, rect.y, 14, 1),
    );
    let value_width = rect.width.saturating_sub(15);
    let (visible, cursor) = if input.is_empty() {
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
        Rect::new(rect.x + 14, rect.y, value_width, 1),
    );
    if focused {
        frame.set_cursor_position(ratatui::layout::Position::new(
            (rect.x + 14 + cursor).min(rect.x + 13 + value_width),
            rect.y,
        ));
    }
    register(&mut app.hits, rect, Action::Focus(field));
}

fn notes_layout(value: &str, cursor: usize, width: usize) -> (Vec<String>, usize, usize) {
    let width = width.max(1);
    let mut lines = vec![String::new()];
    let mut row = 0_usize;
    let mut column = 0_usize;
    let mut index = 0_usize;
    let mut position = if cursor == 0 { Some((0, 0)) } else { None };
    for ch in value.chars() {
        if ch == '\n' {
            index += 1;
            lines.push(String::new());
            row += 1;
            column = 0;
            if cursor == index {
                position = Some((row, column));
            }
            continue;
        }
        let mut buffer = [0_u8; 4];
        let char_width = format::text_width(ch.encode_utf8(&mut buffer));
        if column + char_width > width && column > 0 {
            lines.push(String::new());
            row += 1;
            column = 0;
        }
        lines[row].push(ch);
        column += char_width;
        index += ch.len_utf8();
        if cursor == index {
            position = Some((row, column));
        }
    }
    let (row, column) = position.unwrap_or((row, column));
    (lines, row, column)
}

fn draw_notes(frame: &mut Frame, app: &mut ReleaseApp, area: Rect) {
    let theme = &app.theme;
    let focused = field_focused(app, Field::Notes);
    let hovered = app.hover.as_ref() == Some(&Action::Focus(Field::Notes));
    let label = Rect::new(area.x, area.y, area.width, 1);
    if focused || hovered {
        fill(frame, label, theme.color("surface0"));
    }
    frame.render_widget(
        Paragraph::new(icons::label(icons::BOOK, "发行说明喵~（留空自动生成）")).style(
            Style::default().fg(if focused {
                theme.color("blue")
            } else if hovered {
                theme.color("text")
            } else {
                theme.color("overlay0")
            }),
        ),
        Rect::new(label.x + 1, label.y, label.width.saturating_sub(2), 1),
    );
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused {
            theme.color("blue")
        } else if hovered {
            theme.color("surface2")
        } else {
            theme.color("surface1")
        }));
    let inner = block.inner(Rect::new(
        area.x,
        area.y + 1,
        area.width,
        area.height.saturating_sub(1),
    ));
    frame.render_widget(
        block,
        Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        ),
    );
    let width = inner.width.max(1) as usize;
    let height = inner.height as usize;
    let (all_lines, cursor_row, cursor_column) =
        notes_layout(&app.form.notes.value, app.form.notes.cursor, width);
    let start = if focused {
        cursor_row.saturating_sub(height.saturating_sub(1))
    } else {
        0
    };
    let mut lines: Vec<Line> = if app.form.notes.value.is_empty() {
        vec![Line::from(Span::styled(
            "写点更新内容喵~ 说明为空时按「自动生成说明」开关处理喵~",
            Style::default().fg(theme.color("overlay0")),
        ))]
    } else {
        all_lines
            .iter()
            .skip(start)
            .take(height)
            .map(|line| {
                Line::from(Span::styled(
                    line.clone(),
                    Style::default().fg(theme.color("text")),
                ))
            })
            .collect()
    };
    lines.truncate(height);
    frame.render_widget(Paragraph::new(lines), inner);
    if focused && height > 0 {
        let row = cursor_row.saturating_sub(start).min(height - 1);
        let column = cursor_column.min(width.saturating_sub(1));
        frame.set_cursor_position(ratatui::layout::Position::new(
            inner.x + column as u16,
            inner.y + row as u16,
        ));
    }
    register(
        &mut app.hits,
        Rect::new(area.x, area.y, area.width, area.height),
        Action::Focus(Field::Notes),
    );
}

fn draw_toggles(frame: &mut Frame, app: &mut ReleaseApp, area: Rect) {
    let theme = &app.theme;
    let gap = 1_u16;
    let width = (area.width.saturating_sub(gap)) / 2;
    let items = [
        (Field::Draft, "草稿喵~", app.form.draft),
        (Field::Prerelease, "预发布喵~", app.form.prerelease),
        (
            Field::GenerateNotes,
            "自动生成说明喵~",
            app.form.generate_notes,
        ),
        (Field::Latest, "标记最新喵~", app.form.latest),
    ];
    for (index, (field, label, enabled)) in items.into_iter().enumerate() {
        let column = index % 2;
        let row = index / 2;
        let rect = Rect::new(
            area.x + column as u16 * (width + gap),
            area.y + row as u16,
            width,
            1,
        );
        let action = Action::Toggle(field);
        let hovered = app.hover.as_ref() == Some(&action);
        let focused = field_focused(app, field);
        if focused || hovered {
            fill(
                frame,
                rect,
                if focused {
                    theme.color("surface0")
                } else {
                    theme.color("surface1")
                },
            );
        }
        let icon = if enabled {
            icons::CHECK_SQUARE
        } else {
            icons::SQUARE
        };
        frame.render_widget(
            Paragraph::new(format!("{icon} {label}")).style(Style::default().fg(if enabled {
                theme.color("green")
            } else if focused || hovered {
                theme.color("text")
            } else {
                theme.color("subtext1")
            })),
            Rect::new(rect.x + 1, rect.y, rect.width.saturating_sub(2), 1),
        );
        register(&mut app.hits, rect, action);
    }
}

fn draw_preview(frame: &mut Frame, app: &ReleaseApp, area: Rect) {
    let theme = &app.theme;
    let preview = command_preview(
        app.repos
            .get(app.selected)
            .map(|repo| repo.name_with_owner.as_str())
            .unwrap_or("owner/repo"),
        &app.form,
    );
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.color("surface1")))
        .title(icons::label(icons::TERMINAL, "命令预览喵~"));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines: Vec<Line> = format::wrap_text(&preview, inner.width.max(1) as usize, 3)
        .into_iter()
        .map(|line| {
            Line::from(Span::styled(
                line,
                Style::default().fg(theme.color("overlay0")),
            ))
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_buttons(frame: &mut Frame, app: &mut ReleaseApp, area: Rect) {
    let clear = icons::label(icons::TRASH, "清空喵~");
    let clear_width = format::text_width(&clear) as u16 + 4;
    let clear_rect = Rect::new(area.x, area.y, clear_width, 1);
    draw_button(frame, app, clear_rect, &clear, &Action::Clear, false);
    register(&mut app.hits, clear_rect, Action::Clear);
    let publish = if app.publishing {
        format!(
            "{} 发布中喵~",
            format::spinner(app.started.elapsed().as_millis() as u64 / 120)
        )
    } else {
        icons::label(icons::TAG, "发布喵~")
    };
    let publish_width = format::text_width(&publish) as u16 + 6;
    let publish_rect = Rect::new(
        area.x + area.width.saturating_sub(publish_width),
        area.y,
        publish_width,
        1,
    );
    draw_button(frame, app, publish_rect, &publish, &Action::Publish, true);
    register(&mut app.hits, publish_rect, Action::Publish);
}

fn draw_button(
    frame: &mut Frame,
    app: &ReleaseApp,
    rect: Rect,
    label: &str,
    action: &Action,
    primary: bool,
) {
    let theme = &app.theme;
    let hovered = app.hover.as_ref() == Some(action);
    let bg = if primary {
        if hovered {
            theme.color("sapphire")
        } else {
            theme.color("blue")
        }
    } else if hovered {
        theme.color("surface1")
    } else {
        theme.color("surface0")
    };
    let fg = if primary {
        theme.color("base")
    } else if hovered {
        theme.color("text")
    } else {
        theme.color("subtext1")
    };
    fill(frame, rect, bg);
    frame.render_widget(
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(Style::default().fg(fg).add_modifier(Modifier::BOLD)),
        rect,
    );
    // Hits are registered by the caller through the app owner later.
    let _ = action;
}

fn draw_confirm(frame: &mut Frame, app: &mut ReleaseApp, area: Rect) {
    let theme = app.theme.clone();
    let width = area.width.saturating_sub(12).min(66);
    let height = 10;
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    app.overlay_rect = rect;
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("peach"))),
        rect,
    );
    let repo = app
        .repos
        .get(app.selected)
        .map(|repo| repo.name_with_owner.clone())
        .unwrap_or_else(|| "?".to_string());
    let tag = app.form.tag.value.trim().to_string();
    let mut lines = vec![
        Line::from(Span::styled(
            "确认发布喵~",
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        Line::from(Span::styled(
            format::truncate(&format!("{repo} → {tag}"), width.saturating_sub(4) as usize),
            Style::default().fg(theme.color("blue")),
        )),
        Line::from(Span::styled(
            format!(
                "标签 {tag} · 目标 {} · {}",
                if app.form.target.value.trim().is_empty() {
                    "默认分支"
                } else {
                    app.form.target.value.trim()
                },
                if app.form.draft {
                    "草稿"
                } else if app.form.prerelease {
                    "预发布"
                } else {
                    "正式发行版"
                }
            ),
            Style::default().fg(theme.color("subtext0")),
        )),
        Line::default(),
        Line::from(Span::styled(
            "Enter 确认发布    Esc 取消喵~",
            Style::default().fg(theme.color("overlay0")),
        )),
    ];
    lines.truncate(rect.height.saturating_sub(2) as usize);
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        Rect::new(
            rect.x + 2,
            rect.y + 1,
            rect.width.saturating_sub(4),
            rect.height.saturating_sub(2),
        ),
    );
    let buttons_y = rect.y + rect.height.saturating_sub(2);
    let cancel_width = 14;
    let confirm_width = 18;
    let cancel_rect = Rect::new(rect.x + 2, buttons_y, cancel_width, 1);
    let confirm_rect = Rect::new(
        rect.x + rect.width.saturating_sub(confirm_width + 2),
        buttons_y,
        confirm_width,
        1,
    );
    draw_button(frame, app, cancel_rect, "取消喵~", &Action::Cancel, false);
    draw_button(
        frame,
        app,
        confirm_rect,
        "确认发布喵~",
        &Action::Confirm,
        true,
    );
    register(&mut app.hits, cancel_rect, Action::Cancel);
    register(&mut app.hits, confirm_rect, Action::Confirm);
}

fn draw_help(frame: &mut Frame, app: &mut ReleaseApp, area: Rect) {
    let theme = app.theme.clone();
    let width = area.width.saturating_sub(10).min(74);
    let height = area.height.saturating_sub(6).min(20);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    app.overlay_rect = rect;
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("blue"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    let mut lines = vec![
        Line::from(Span::styled(
            "个人发行版发布喵~",
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        Line::from("j/k 或 ↑/↓   仓库列表选择"),
        Line::from("g/G          仓库列表首尾"),
        Line::from("r            重新读取 gh 仓库列表"),
        Line::from("Tab/BackTab  切换仓库、标签、标题、目标、说明、开关"),
        Line::from("Enter        文本字段下一项；说明里换行；开关切换"),
        Line::from("Ctrl+S       提交发布（会再确认一次）"),
        Line::from("鼠标         点仓库/字段/开关/按钮，悬停高亮"),
        Line::from("Esc          关闭帮助、取消确认或退出"),
        Line::default(),
        Line::from(Span::styled(
            "说明留空且关闭「自动生成说明」时不会提交，避免 gh 进入交互喵~",
            Style::default().fg(theme.color("overlay0")),
        )),
    ];
    lines.truncate(inner.height.saturating_sub(1) as usize);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(theme.color("subtext0"))),
        inner,
    );
    let close_rect = Rect::new(inner.x, inner.y + inner.height.saturating_sub(1), 14, 1);
    draw_button(frame, app, close_rect, "关掉喵~", &Action::CloseHelp, true);
    register(&mut app.hits, close_rect, Action::CloseHelp);
}

impl ReleaseApp {
    fn reload_repos(&mut self) {
        if self.repos_loading {
            return;
        }
        self.repos_loading = true;
        self.set_status(StatusKind::Info, "正在用 gh 读仓库列表喵~...");
        let tx = self.repo_tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(load_repos());
        });
    }

    fn drain_repos(&mut self) {
        while let Ok(result) = self.repo_rx.try_recv() {
            self.repos_loading = false;
            match result {
                Ok(repos) => {
                    let count = repos.len();
                    let current = self
                        .repos
                        .get(self.selected)
                        .map(|repo| repo.name_with_owner.clone());
                    self.repos = repos;
                    self.selected = current
                        .and_then(|name| {
                            self.repos
                                .iter()
                                .position(|repo| repo.name_with_owner == name)
                        })
                        .unwrap_or(0);
                    self.offset = 0;
                    self.clamp_selected();
                    self.set_status(
                        StatusKind::Ok,
                        &format!("从 gh 读到 {count} 个仓库喵~ 按 j/k 选仓库喵~"),
                    );
                }
                Err(error) => {
                    self.repos.clear();
                    self.selected = 0;
                    self.offset = 0;
                    self.set_status(StatusKind::Error, &error);
                }
            }
        }
    }

    fn clamp_selected(&mut self) {
        if self.repos.is_empty() {
            self.selected = 0;
            self.offset = 0;
        } else {
            self.selected = self.selected.min(self.repos.len() - 1);
        }
    }

    fn set_status(&mut self, kind: StatusKind, text: &str) {
        self.status_kind = kind;
        self.status = text.to_string();
    }

    fn drain(&mut self) {
        self.drain_repos();
        while let Ok(result) = self.rx.try_recv() {
            self.publishing = false;
            match result {
                Ok(output) => {
                    self.clear_form();
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

    fn clear_form(&mut self) {
        self.form = Form::default();
        self.set_status(StatusKind::Info, "表单清空啦喵~");
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

    fn hit_at(&self, column: u16, row: u16) -> Option<&Action> {
        self.hits
            .iter()
            .rev()
            .find(|hit| {
                column >= hit.rect.x
                    && column < hit.rect.x.saturating_add(hit.rect.width)
                    && row >= hit.rect.y
                    && row < hit.rect.y.saturating_add(hit.rect.height)
            })
            .map(|hit| &hit.action)
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::Moved => {
                self.hover = self.hit_at(mouse.column, mouse.row).cloned();
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                if self.show_help || self.confirming {
                    return;
                }
                let delta = if matches!(mouse.kind, MouseEventKind::ScrollUp) {
                    -1
                } else {
                    1
                };
                if contains(self.list_rect, mouse.column, mouse.row) && !self.repos.is_empty() {
                    self.selected = (self.selected as i32 + delta)
                        .clamp(0, self.repos.len() as i32 - 1)
                        as usize;
                    self.form.field = Field::Repos;
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if self.publishing && !self.show_help && !self.confirming {
                    return;
                }
                if self.show_help || self.confirming {
                    if let Some(action) = self.hit_at(mouse.column, mouse.row).cloned() {
                        self.handle_action(action);
                    } else if !contains(self.overlay_rect, mouse.column, mouse.row) {
                        self.show_help = false;
                        self.confirming = false;
                        self.set_status(StatusKind::Info, "取消啦喵~");
                    }
                    return;
                }
                if let Some(action) = self.hit_at(mouse.column, mouse.row).cloned() {
                    self.handle_action(action);
                }
            }
            _ => {}
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::SelectRepo(index) => {
                self.selected = index.min(self.repos.len().saturating_sub(1));
                self.form.field = Field::Repos;
            }
            Action::Focus(field) => {
                self.form.field = field;
                match field {
                    Field::Tag => self.form.tag.end(),
                    Field::Title => self.form.title.end(),
                    Field::Target => self.form.target.end(),
                    Field::Notes => self.form.notes.end(),
                    _ => {}
                }
            }
            Action::Toggle(field) => {
                self.form.field = field;
                match field {
                    Field::Draft => self.form.draft = !self.form.draft,
                    Field::Prerelease => self.form.prerelease = !self.form.prerelease,
                    Field::GenerateNotes => self.form.generate_notes = !self.form.generate_notes,
                    Field::Latest => self.form.latest = !self.form.latest,
                    _ => {}
                }
            }
            Action::Publish => self.submit(),
            Action::Clear => self.clear_form(),
            Action::Confirm => {
                self.confirming = false;
                self.publish();
            }
            Action::Cancel => {
                self.confirming = false;
                self.set_status(StatusKind::Info, "取消发布啦喵~");
            }
            Action::CloseHelp => self.show_help = false,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.show_help {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
            ) {
                self.show_help = false;
            }
            return;
        }
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
        if key.code == KeyCode::Char('?') {
            self.show_help = true;
            self.overlay_rect = Rect::default();
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
            Field::Draft => {
                if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                    self.form.draft = !self.form.draft;
                }
            }
            Field::Prerelease => {
                if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                    self.form.prerelease = !self.form.prerelease;
                }
            }
            Field::GenerateNotes => {
                if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                    self.form.generate_notes = !self.form.generate_notes;
                }
            }
            Field::Latest => {
                if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                    self.form.latest = !self.form.latest;
                }
            }
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
        if self.form.notes.value.trim().is_empty() && !self.form.generate_notes {
            self.form.field = Field::GenerateNotes;
            self.set_status(StatusKind::Error, "说明空着时要打开「自动生成说明」喵~");
            return;
        }
        self.confirming = true;
        self.hover = None;
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

fn contains(rect: Rect, x: u16, y: u16) -> bool {
    rect.width > 0
        && rect.height > 0
        && x >= rect.x
        && x < rect.x.saturating_add(rect.width)
        && y >= rect.y
        && y < rect.y.saturating_add(rect.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_cursor_tracks_newlines_and_wrap() {
        let (lines, row, column) = notes_layout("abc\ndef", 4, 10);
        assert_eq!(lines, vec!["abc".to_string(), "def".to_string()]);
        assert_eq!((row, column), (1, 0));
        let (_, row, column) = notes_layout("abc\ndef", 8, 10);
        assert_eq!((row, column), (1, 3));
        let (_, row, column) = notes_layout("abcd", 4, 3);
        assert_eq!((row, column), (1, 1));
        let (_, row, column) = notes_layout("中文", 6, 10);
        assert_eq!((row, column), (0, 4));
    }

    #[test]
    fn builds_release_args() {
        let form = Form {
            tag: TextInput::new("v0.2.2"),
            title: TextInput::new("Release 0.2.2"),
            target: TextInput::new("main"),
            draft: true,
            prerelease: true,
            generate_notes: false,
            notes: TextInput::new("hello"),
            latest: true,
            ..Form::default()
        };
        let args = release_args("owner/repo", &form);
        assert_eq!(args[0], "release");
        assert_eq!(args[1], "create");
        assert_eq!(args[2], "v0.2.2");
        assert!(args.contains(&"--repo".to_string()));
        assert!(args.contains(&"--title".to_string()));
        assert!(args.contains(&"--notes".to_string()));
        assert!(args.contains(&"hello".to_string()));
        assert!(!args.contains(&"--generate-notes".to_string()));
        assert!(args.contains(&"--draft".to_string()));
        assert!(args.contains(&"--prerelease".to_string()));
        assert!(args.contains(&"--latest".to_string()));
        let empty = Form {
            tag: TextInput::new("v1"),
            ..Form::default()
        };
        let args = release_args("owner/repo", &empty);
        assert!(args.contains(&"--generate-notes".to_string()));
        assert!(command_preview("owner/repo", &empty).starts_with("gh release create v1"));
    }

    #[test]
    fn parses_gh_repo_json_and_fields() {
        let repos: Vec<GhRepo> = serde_json::from_str(
            r#"[{"nameWithOwner":"a/b","description":null,"isPrivate":true,"stargazerCount":5,"forkCount":2,"updatedAt":"2026-01-01T00:00:00Z"}]"#,
        )
        .unwrap();
        assert_eq!(repos[0].name_with_owner, "a/b");
        assert_eq!(repos[0].owner(), "a");
        assert_eq!(repos[0].name(), "b");
        assert!(repos[0].is_private);
        assert_eq!(repos[0].stargazer_count, 5);
        assert_eq!(Field::Repos.next(), Field::Tag);
        assert_eq!(Field::Repos.prev(), Field::Submit);
    }
}
