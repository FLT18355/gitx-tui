use crate::format;
use crate::icons;
use crate::local_app::{LocalApp, LocalPane};
use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

const MIN_WIDTH: u16 = 64;
const MIN_HEIGHT: u16 = 16;

pub fn draw(frame: &mut Frame, app: &LocalApp) {
    let area = frame.area();
    let theme = app.theme.clone();
    fill(frame, area, theme.color("base"));
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_center(
            frame,
            &theme,
            area,
            "终端再大一点嘛喵~",
            "本地仓库页至少要 64 x 16 才摆得下喵~",
        );
        return;
    }
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .split(area);
    draw_header(frame, app, &theme, chunks[0]);
    draw_body(frame, app, &theme, chunks[1]);
    draw_footer(frame, app, &theme, chunks[2]);
    if app.commit_input.is_some() {
        draw_commit_input(frame, app, &theme, area);
    }
}

fn draw_header(frame: &mut Frame, app: &LocalApp, theme: &Theme, area: Rect) {
    fill(frame, area, theme.color("mantle"));
    let brand = Line::from(vec![
        Span::styled(
            format!("{} ", icons::TERMINAL),
            Style::default().fg(theme.color("green")),
        ),
        Span::styled(
            "gxt",
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  本地仓库喵~",
            Style::default().fg(theme.color("subtext0")),
        ),
        Span::styled(
            format!("  {}", app.repo.display_root()),
            Style::default().fg(theme.color("overlay0")),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(brand),
        Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1),
    );
    let mut branch = vec![
        Span::styled(
            format!("{} ", icons::BRANCH),
            Style::default().fg(theme.color("blue")),
        ),
        Span::styled(
            app.repo.branch.clone(),
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if !app.repo.upstream.is_empty() {
        branch.push(Span::styled(
            format!("  → {}", app.repo.upstream),
            Style::default().fg(theme.color("overlay0")),
        ));
    }
    if app.repo.ahead > 0 {
        branch.push(Span::styled(
            format!("  ↑{}", app.repo.ahead),
            Style::default().fg(theme.color("green")),
        ));
    }
    if app.repo.behind > 0 {
        branch.push(Span::styled(
            format!("  ↓{}", app.repo.behind),
            Style::default().fg(theme.color("peach")),
        ));
    }
    branch.push(Span::styled(
        format!(
            "   {} 个改动    {} 个提交    {} 个分支",
            app.repo.files.len(),
            app.repo.commits.len(),
            app.repo.branches.len()
        ),
        Style::default().fg(theme.color("overlay0")),
    ));
    frame.render_widget(
        Paragraph::new(Line::from(branch)),
        Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
    );
    fill(
        frame,
        Rect::new(area.x, area.y + 2, area.width, 1),
        theme.color("surface0"),
    );
}

fn draw_body(frame: &mut Frame, app: &LocalApp, theme: &Theme, area: Rect) {
    let branch_width = if area.width >= 100 { 24 } else { 18 };
    let commit_width = if area.width >= 110 {
        40
    } else if area.width >= 90 {
        34
    } else {
        27
    };
    let columns = Layout::horizontal([
        Constraint::Length(branch_width),
        Constraint::Min(20),
        Constraint::Length(commit_width),
    ])
    .split(area);
    draw_branches(frame, app, theme, columns[0]);
    draw_status(frame, app, theme, columns[1]);
    draw_commits(frame, app, theme, columns[2]);
}

fn panel(frame: &mut Frame, theme: &Theme, area: Rect, title: &str, focused: bool) -> Rect {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Paragraph::new(title).style(if focused {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("text"))
        }),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    fill(
        frame,
        Rect::new(area.x, area.y + 1, area.width, 1),
        theme.color("surface0"),
    );
    Rect::new(
        area.x + 1,
        area.y + 2,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

fn draw_branches(frame: &mut Frame, app: &LocalApp, theme: &Theme, area: Rect) {
    let body = panel(
        frame,
        theme,
        area,
        &icons::label(icons::BRANCH, "分支喵~"),
        app.pane == LocalPane::Branches,
    );
    if body.height == 0 {
        return;
    }
    if app.repo.branches.is_empty() {
        draw_center(frame, theme, body, "还没有分支喵~", "先提交一次就有啦喵~");
        return;
    }
    let rows = body.height as usize;
    let selected = app
        .branch_selected
        .min(app.repo.branches.len().saturating_sub(1));
    let start = selected.saturating_sub(rows.saturating_sub(1));
    let end = (start + rows).min(app.repo.branches.len());
    for (offset, index) in (start..end).enumerate() {
        let branch = &app.repo.branches[index];
        let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
        if app.pane == LocalPane::Branches && index == selected {
            fill(frame, row, theme.color("surface0"));
        }
        let marker = if branch.current { "*" } else { " " };
        let text = format!("{} {}  {}", marker, branch.name, branch.upstream);
        frame.render_widget(
            Paragraph::new(format::truncate(&text, body.width as usize)).style(if branch.current {
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.color("subtext0"))
            }),
            row,
        );
    }
}

fn draw_status(frame: &mut Frame, app: &LocalApp, theme: &Theme, area: Rect) {
    let title = format!(
        "{} 改动喵~  暂存 {} · 未暂存 {}",
        icons::FILE,
        app.repo.staged_count(),
        app.repo.unstaged_count()
    );
    let body = panel(frame, theme, area, &title, app.pane == LocalPane::Status);
    if body.height == 0 {
        return;
    }
    if app.repo.files.is_empty() {
        draw_center(
            frame,
            theme,
            body,
            "工作区干干净净喵~",
            "改点文件再按 r 刷新看看嘛喵~",
        );
        return;
    }
    let rows = body.height as usize;
    let selected = app
        .file_selected
        .min(app.repo.files.len().saturating_sub(1));
    let start = selected.saturating_sub(rows.saturating_sub(1));
    let end = (start + rows).min(app.repo.files.len());
    for (offset, index) in (start..end).enumerate() {
        let file = &app.repo.files[index];
        let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
        if app.pane == LocalPane::Status && index == selected {
            fill(frame, row, theme.color("surface0"));
        }
        let mark = format!(
            "{}{}",
            if file.staged() { "S" } else { " " },
            if file.unstaged() { "U" } else { " " }
        );
        let color = if file.untracked() {
            theme.color("overlay0")
        } else if file.staged() {
            theme.color("green")
        } else {
            theme.color("peach")
        };
        let text = format!("{} {:<6} {}", mark, file.status_label(), file.path);
        frame.render_widget(
            Paragraph::new(format::truncate(&text, body.width as usize))
                .style(Style::default().fg(color)),
            row,
        );
    }
}

fn draw_commits(frame: &mut Frame, app: &LocalApp, theme: &Theme, area: Rect) {
    let body = panel(
        frame,
        theme,
        area,
        &icons::label(icons::CLOCK, "提交喵~"),
        app.pane == LocalPane::Commits,
    );
    if body.height == 0 {
        return;
    }
    if app.repo.commits.is_empty() {
        draw_center(
            frame,
            theme,
            body,
            "还没有提交喵~",
            "暂存改动后按 c 写第一条提交吧喵~",
        );
        return;
    }
    let rows = (body.height / 2).max(1) as usize;
    let selected = app
        .commit_selected
        .min(app.repo.commits.len().saturating_sub(1));
    let start = selected.saturating_sub(rows.saturating_sub(1));
    let end = (start + rows).min(app.repo.commits.len());
    for (offset, index) in (start..end).enumerate() {
        let commit = &app.repo.commits[index];
        let y = body.y + offset as u16 * 2;
        if y + 1 >= body.y + body.height {
            break;
        }
        let row = Rect::new(body.x, y, body.width, 2);
        if app.pane == LocalPane::Commits && index == selected {
            fill(frame, row, theme.color("surface0"));
        }
        let text = Line::from(vec![
            Span::styled(
                format!("{} ", commit.short),
                Style::default().fg(theme.color("blue")),
            ),
            Span::styled(
                format::truncate(
                    &format::clean_text(&commit.summary),
                    body.width.saturating_sub(10) as usize,
                ),
                Style::default().fg(theme.color("text")),
            ),
        ]);
        frame.render_widget(Paragraph::new(text), Rect::new(row.x, y, body.width, 1));
        frame.render_widget(
            Paragraph::new(format::truncate(
                &format!("  {} · {}", commit.author, commit.when),
                body.width as usize,
            ))
            .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(row.x, y + 1, body.width, 1),
        );
    }
}

fn draw_footer(frame: &mut Frame, app: &LocalApp, theme: &Theme, area: Rect) {
    fill(frame, area, theme.color("crust"));
    let (icon, color) = if app.message_error {
        (icons::WARNING, theme.color("red"))
    } else {
        (icons::INFO, theme.color("subtext0"))
    };
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("{icon}  {}", app.message),
            area.width.saturating_sub(4) as usize,
        ))
        .style(Style::default().fg(color)),
        Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            "Tab 换面板  j/k 挑  s 暂存  u 取消暂存  c 提交  r 刷新  q 退出喵~",
            area.width.saturating_sub(4) as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
    );
}

fn draw_commit_input(frame: &mut Frame, app: &LocalApp, theme: &Theme, area: Rect) {
    let Some(input) = app.commit_input.as_ref() else {
        return;
    };
    let width = area.width.saturating_sub(12).min(64);
    let height = 7.min(area.height.saturating_sub(4));
    if width < 30 || height < 5 {
        return;
    }
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("green"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    frame.render_widget(
        Paragraph::new("提交信息喵~").style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let input_rect = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    fill(frame, input_rect, theme.color("surface0"));
    let empty = input.is_empty();
    let shown = if empty {
        "写点什么喵~".to_string()
    } else {
        input.value.clone()
    };
    frame.render_widget(
        Paragraph::new(format::truncate(&shown, inner.width as usize)).style(if empty {
            Style::default().fg(theme.color("overlay0"))
        } else {
            Style::default().fg(theme.color("text"))
        }),
        input_rect,
    );
    let cursor_x = if empty {
        inner.x
    } else {
        inner.x + (input.cursor as u16).min(inner.width.saturating_sub(1))
    };
    frame.set_cursor_position(Position::new(cursor_x, inner.y + 1));
    frame.render_widget(
        Paragraph::new("Enter 提交   Esc 取消喵~")
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
    );
}

fn draw_center(frame: &mut Frame, theme: &Theme, area: Rect, title: &str, detail: &str) {
    let lines = vec![
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            detail,
            Style::default().fg(theme.color("overlay0")),
        )),
    ];
    let height = lines.len() as u16;
    let rect = Rect::new(
        area.x + 2,
        area.y + area.height.saturating_sub(height) / 2,
        area.width.saturating_sub(4),
        height.min(area.height),
    );
    frame.render_widget(
        Paragraph::new(Text::from(lines)).alignment(Alignment::Center),
        rect,
    );
}

fn fill(frame: &mut Frame, rect: Rect, color: Color) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    frame.render_widget(Block::new().style(Style::default().bg(color)), rect);
}
