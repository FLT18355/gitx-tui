use crate::format;
use crate::icons;
use crate::local::StatusFile;
use crate::local_app::{LocalAction, LocalApp, LocalLayout, LocalPane, PromptKind};
use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

const MIN_WIDTH: u16 = 80;
const MIN_HEIGHT: u16 = 22;

pub fn draw(frame: &mut Frame, app: &mut LocalApp) {
    let area = frame.area();
    let theme = app.theme.clone();
    app.layout = LocalLayout::default();
    app.hits.clear();
    fill(frame, area, theme.color("base"));
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_center(
            frame,
            &theme,
            area,
            "终端再大一点嘛喵~",
            "本地仓库页至少要 80 x 22 才摆得下喵~",
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
    if app.prompt.is_some() {
        draw_prompt(frame, app, &theme, area);
    }
    if app.help {
        draw_help(frame, &theme, area);
    }
}

fn draw_header(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    fill(frame, area, theme.color("mantle"));
    let brand_hover = app.pointer.is_some_and(|(_, y)| y == area.y);
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
            format!(
                "  {}",
                format::truncate(
                    &app.repo.display_root(),
                    area.width.saturating_sub(24) as usize
                )
            ),
            Style::default().fg(if brand_hover {
                theme.color("blue")
            } else {
                theme.color("overlay0")
            }),
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
            format!(" → {}", app.repo.upstream),
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
    if let Some(busy) = app.busy.as_deref() {
        let frame_no = (app.started_at.elapsed().as_millis() / 120) as u64;
        branch.push(Span::styled(
            format!("  {} {busy}", format::spinner(frame_no)),
            Style::default().fg(theme.color("peach")),
        ));
    }
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

fn draw_body(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    let sidebar_width = (area.width / 4).clamp(20, 28);
    let remaining = area.width.saturating_sub(sidebar_width);
    let diff_width = (remaining * 45 / 100).max(20);
    let middle_width = remaining.saturating_sub(diff_width);
    let columns = Layout::horizontal([
        Constraint::Length(sidebar_width),
        Constraint::Length(middle_width),
        Constraint::Min(diff_width),
    ])
    .split(area);

    draw_sidebar(frame, app, theme, columns[0]);

    let middle = Layout::vertical([
        Constraint::Percentage(55),
        Constraint::Length(1),
        Constraint::Min(8),
    ])
    .split(columns[1]);
    draw_status(frame, app, theme, middle[0]);
    fill(
        frame,
        Rect::new(middle[1].x, middle[1].y, columns[1].width, 1),
        theme.color("surface0"),
    );
    draw_commits(frame, app, theme, middle[2]);

    draw_diff(frame, app, theme, columns[2]);
}

fn draw_sidebar(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let inner = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    let rows = Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Min(3),
    ])
    .split(inner);
    draw_branches(frame, app, theme, rows[0]);
    fill(frame, rows[1], theme.color("surface0"));
    draw_tags(frame, app, theme, rows[2]);
    fill(frame, rows[3], theme.color("surface0"));
    draw_stashes(frame, app, theme, rows[4]);
}

#[allow(clippy::too_many_arguments)]
fn section(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    icon: &str,
    title: &str,
    meta: &str,
    focused: bool,
    hovered: bool,
) -> Rect {
    if area.width < 4 || area.height == 0 {
        return Rect::default();
    }
    if hovered && !focused {
        fill(
            frame,
            Rect::new(area.x, area.y, area.width, 1),
            theme.color("surface0"),
        );
    }
    let accent = if focused {
        theme.color("blue")
    } else if hovered {
        theme.color("text")
    } else {
        theme.color("overlay0")
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{icon} "), Style::default().fg(accent)),
            Span::styled(
                title.to_string(),
                if focused {
                    Style::default()
                        .fg(theme.color("blue"))
                        .add_modifier(Modifier::BOLD)
                } else if hovered {
                    Style::default()
                        .fg(theme.color("text"))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.color("text"))
                },
            ),
        ])),
        Rect::new(area.x, area.y, area.width, 1),
    );
    if !meta.is_empty() {
        frame.render_widget(
            Paragraph::new(meta.to_string())
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x, area.y, area.width, 1),
        );
    }
    Rect::new(
        area.x,
        area.y + 1,
        area.width,
        area.height.saturating_sub(1),
    )
}

fn page_label(total: usize, selected: usize, rows: usize) -> String {
    if total == 0 || rows == 0 {
        return String::new();
    }
    let pages = total.div_ceil(rows);
    let page = (selected / rows + 1).min(pages);
    format!("{page}/{pages}")
}

fn scrollbar(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    total: usize,
    rows: usize,
    start: usize,
) {
    if rows == 0 || total <= rows || area.width == 0 || area.height == 0 {
        return;
    }
    let x = area.x + area.width - 1;
    for y in 0..area.height {
        frame.render_widget(
            Paragraph::new("|").style(Style::default().fg(theme.color("surface0"))),
            Rect::new(x, area.y + y, 1, 1),
        );
    }
    let thumb =
        ((area.height as f32 * rows as f32 / total as f32).max(1.0) as u16).min(area.height);
    let max_start = total.saturating_sub(rows).max(1);
    let offset =
        (start as f32 / max_start as f32 * area.height.saturating_sub(thumb) as f32) as u16;
    for y in 0..thumb {
        frame.render_widget(
            Paragraph::new("|").style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(x, area.y + offset + y, 1, 1),
        );
    }
}

fn draw_branches(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    let meta = page_label(
        app.repo.branches.len(),
        app.branch_selected,
        app.branch_rows,
    );
    let body = section(
        frame,
        theme,
        area,
        icons::BRANCH,
        "分支喵~",
        &meta,
        app.pane == LocalPane::Branches,
        app.hover_pane == Some(LocalPane::Branches),
    );
    if area.width > 0 {
        app.hits.push((
            Rect::new(area.x, area.y, area.width, 1),
            LocalAction::Focus(LocalPane::Branches),
        ));
    }
    app.layout.branches = body;
    app.branch_rows = body.height.max(1) as usize;
    if body.height == 0 {
        return;
    }
    if app.repo.branches.is_empty() {
        if app.hover_pane == Some(LocalPane::Branches) {
            fill(frame, body, theme.color("surface0"));
        }
        draw_center(frame, theme, body, "还没有分支喵~", "先提交一次就有啦喵~");
        return;
    }
    let rows = app.branch_rows;
    let selected = app
        .branch_selected
        .min(app.repo.branches.len().saturating_sub(1));
    let start = (selected / rows) * rows;
    let end = (start + rows).min(app.repo.branches.len());
    let text_width = body.width.saturating_sub(1) as usize;
    for (offset, index) in (start..end).enumerate() {
        let branch = &app.repo.branches[index];
        let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
        if app.pane == LocalPane::Branches && index == selected {
            fill(frame, row, theme.color("surface1"));
        } else if app.hover_pane == Some(LocalPane::Branches) && app.hover_row == Some(index) {
            fill(frame, row, theme.color("surface0"));
        }
        let marker = if branch.current { "*" } else { " " };
        let line = Line::from(vec![
            Span::styled(marker, Style::default().fg(theme.color("green"))),
            Span::styled(
                format!(
                    " {}",
                    format::truncate(
                        &format::clean_text(&branch.name),
                        text_width.saturating_sub(2)
                    )
                ),
                if branch.current {
                    Style::default()
                        .fg(theme.color("blue"))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.color("text"))
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }
    scrollbar(frame, theme, body, app.repo.branches.len(), rows, start);
}

fn draw_tags(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    let meta = page_label(app.repo.tags.len(), app.tag_selected, app.tag_rows);
    let body = section(
        frame,
        theme,
        area,
        icons::TAG,
        "标签喵~",
        &meta,
        app.pane == LocalPane::Tags,
        app.hover_pane == Some(LocalPane::Tags),
    );
    if area.width > 0 {
        app.hits.push((
            Rect::new(area.x, area.y, area.width, 1),
            LocalAction::Focus(LocalPane::Tags),
        ));
    }
    app.layout.tags = body;
    app.tag_rows = body.height.max(1) as usize;
    if body.height == 0 {
        return;
    }
    if app.repo.tags.is_empty() {
        if app.hover_pane == Some(LocalPane::Tags) {
            fill(frame, body, theme.color("surface0"));
        }
        draw_center(frame, theme, body, "还没有标签喵~", "按 t 建一个试试嘛喵~");
        return;
    }
    let rows = app.tag_rows;
    let selected = app.tag_selected.min(app.repo.tags.len().saturating_sub(1));
    let start = (selected / rows) * rows;
    let end = (start + rows).min(app.repo.tags.len());
    let text_width = body.width.saturating_sub(3) as usize;
    for (offset, index) in (start..end).enumerate() {
        let tag = &app.repo.tags[index];
        let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
        if app.pane == LocalPane::Tags && index == selected {
            fill(frame, row, theme.color("surface1"));
        } else if app.hover_pane == Some(LocalPane::Tags) && app.hover_row == Some(index) {
            fill(frame, row, theme.color("surface0"));
        }
        let line = Line::from(vec![
            Span::styled(
                format!("{} ", icons::TAG),
                Style::default().fg(theme.color("peach")),
            ),
            Span::styled(
                format::truncate(&format::clean_text(&tag.name), text_width),
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }
    scrollbar(frame, theme, body, app.repo.tags.len(), rows, start);
}

fn draw_status(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    let meta = format!(
        "暂{} · 未{} · {}",
        app.repo.staged_count(),
        app.repo.unstaged_count(),
        page_label(app.repo.files.len(), app.file_selected, app.status_rows)
    );
    let body = section(
        frame,
        theme,
        area,
        icons::FILE,
        "改动喵~",
        &meta,
        app.pane == LocalPane::Status,
        app.hover_pane == Some(LocalPane::Status),
    );
    if area.width > 0 {
        app.hits.push((
            Rect::new(area.x, area.y, area.width, 1),
            LocalAction::Focus(LocalPane::Status),
        ));
    }
    app.layout.status = body;
    app.status_rows = body.height.max(1) as usize;
    if body.height == 0 {
        return;
    }
    if app.repo.files.is_empty() {
        if app.hover_pane == Some(LocalPane::Status) {
            fill(frame, body, theme.color("surface0"));
        }
        draw_center(
            frame,
            theme,
            body,
            "工作区干干净净喵~",
            "改点文件再按 r 刷新看看嘛喵~",
        );
        return;
    }
    let rows = app.status_rows;
    let selected = app
        .file_selected
        .min(app.repo.files.len().saturating_sub(1));
    let start = (selected / rows) * rows;
    let end = (start + rows).min(app.repo.files.len());
    let text_width = body.width.saturating_sub(10) as usize;
    for (offset, index) in (start..end).enumerate() {
        let file = &app.repo.files[index];
        let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
        if app.pane == LocalPane::Status && index == selected {
            fill(frame, row, theme.color("surface1"));
        } else if app.hover_pane == Some(LocalPane::Status) && app.hover_row == Some(index) {
            fill(frame, row, theme.color("surface0"));
        }
        let (badge, color) = status_badge(file, theme);
        let line = Line::from(vec![
            Span::styled(format!("{badge} "), Style::default().fg(color)),
            Span::styled(
                format!("{:<6}", file.status_label()),
                Style::default().fg(color),
            ),
            Span::styled(
                format::truncate(&format::clean_text(&file.path), text_width),
                Style::default().fg(theme.color("text")),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }
    scrollbar(frame, theme, body, app.repo.files.len(), rows, start);
}

fn status_badge(file: &StatusFile, theme: &Theme) -> (&'static str, Color) {
    if file.untracked() {
        ("?", theme.color("overlay0"))
    } else if file.staged() && file.unstaged() {
        ("±", theme.color("peach"))
    } else if file.staged() {
        ("S", theme.color("green"))
    } else {
        ("U", theme.color("peach"))
    }
}

fn draw_diff(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    let active = app.pane == LocalPane::Diff || app.hover_pane == Some(LocalPane::Diff);
    let border = if active {
        theme.color("blue")
    } else {
        theme.color("surface0")
    };
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(border)),
        area,
    );
    app.layout.diff = area;
    if area.width > 0 && area.height > 0 {
        app.hits.push((area, LocalAction::Focus(LocalPane::Diff)));
    }
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(3),
        area.height.saturating_sub(2),
    );
    if inner.width < 4 || inner.height < 4 {
        return;
    }
    let total = app.diff_text.lines().count();
    let rows = inner.height.saturating_sub(2) as usize;
    let page = if total > rows && rows > 0 {
        let pages = total.div_ceil(rows);
        format!(
            "{}/{}",
            (app.diff_scroll as usize / rows + 1).min(pages),
            pages
        )
    } else {
        String::new()
    };
    frame.render_widget(
        Paragraph::new(format::truncate(
            &app.diff_title,
            inner.width.saturating_sub(page.len() as u16 + 3) as usize,
        ))
        .style(
            Style::default()
                .fg(if active {
                    theme.color("blue")
                } else {
                    theme.color("text")
                })
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    if !page.is_empty() {
        frame.render_widget(
            Paragraph::new(page)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
    }
    fill(
        frame,
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
        theme.color("surface0"),
    );
    let body = Rect::new(
        inner.x,
        inner.y + 2,
        inner.width,
        inner.height.saturating_sub(2),
    );
    app.diff_scroll_max = total.saturating_sub(body.height as usize) as u16;
    app.diff_scroll = app.diff_scroll.min(app.diff_scroll_max);
    if body.height == 0 {
        return;
    }
    let lines: Vec<Line> = app
        .diff_text
        .lines()
        .map(|line| {
            let style = if line.starts_with("+++") || line.starts_with("---") {
                Style::default()
                    .fg(theme.color("overlay0"))
                    .add_modifier(Modifier::BOLD)
            } else if line.starts_with('+') {
                Style::default().fg(theme.color("green"))
            } else if line.starts_with('-') {
                Style::default().fg(theme.color("red"))
            } else if line.starts_with("@@") {
                Style::default().fg(theme.color("blue"))
            } else if line.starts_with("commit ")
                || line.starts_with("Author:")
                || line.starts_with("Date:")
            {
                Style::default().fg(theme.color("peach"))
            } else {
                Style::default().fg(theme.color("subtext0"))
            };
            Line::from(Span::styled(line, style))
        })
        .collect();
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((app.diff_scroll, 0)),
        body,
    );
}

fn draw_commits(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    let meta = page_label(app.repo.commits.len(), app.commit_selected, app.commit_rows);
    let body = section(
        frame,
        theme,
        area,
        icons::CLOCK,
        "提交喵~",
        &meta,
        app.pane == LocalPane::Commits,
        app.hover_pane == Some(LocalPane::Commits),
    );
    if area.width > 0 {
        app.hits.push((
            Rect::new(area.x, area.y, area.width, 1),
            LocalAction::Focus(LocalPane::Commits),
        ));
    }
    app.layout.commits = body;
    app.commit_rows = (body.height / 2).max(1) as usize;
    if body.height == 0 {
        return;
    }
    if app.repo.commits.is_empty() {
        if app.hover_pane == Some(LocalPane::Commits) {
            fill(frame, body, theme.color("surface0"));
        }
        draw_center(
            frame,
            theme,
            body,
            "还没有提交喵~",
            "暂存改动后按 c 写第一条提交吧喵~",
        );
        return;
    }
    let rows = app.commit_rows;
    let selected = app
        .commit_selected
        .min(app.repo.commits.len().saturating_sub(1));
    let start = (selected / rows) * rows;
    let end = (start + rows).min(app.repo.commits.len());
    for (offset, index) in (start..end).enumerate() {
        let commit = &app.repo.commits[index];
        let y = body.y + offset as u16 * 2;
        if y + 1 >= body.y + body.height {
            break;
        }
        let row = Rect::new(body.x, y, body.width, 2);
        if app.pane == LocalPane::Commits && index == selected {
            fill(frame, row, theme.color("surface1"));
        } else if app.hover_pane == Some(LocalPane::Commits) && app.hover_row == Some(index) {
            fill(frame, row, theme.color("surface0"));
        }
        let line = Line::from(vec![
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
        frame.render_widget(Paragraph::new(line), Rect::new(row.x, y, body.width, 1));
        frame.render_widget(
            Paragraph::new(format::truncate(
                &format!("  {} · {}", commit.author, commit.when),
                body.width as usize,
            ))
            .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(row.x, y + 1, body.width, 1),
        );
    }
    scrollbar(frame, theme, body, app.repo.commits.len(), rows, start);
}

fn draw_stashes(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    let meta = page_label(app.repo.stashes.len(), app.stash_selected, app.stash_rows);
    let body = section(
        frame,
        theme,
        area,
        icons::ARCHIVE,
        "stash喵~",
        &meta,
        app.pane == LocalPane::Stashes,
        app.hover_pane == Some(LocalPane::Stashes),
    );
    if area.width > 0 {
        app.hits.push((
            Rect::new(area.x, area.y, area.width, 1),
            LocalAction::Focus(LocalPane::Stashes),
        ));
    }
    app.layout.stashes = body;
    app.stash_rows = body.height.max(1) as usize;
    if body.height == 0 {
        return;
    }
    if app.repo.stashes.is_empty() {
        if app.hover_pane == Some(LocalPane::Stashes) {
            fill(frame, body, theme.color("surface0"));
        }
        draw_center(
            frame,
            theme,
            body,
            "还没有 stash 喵~",
            "按 z 把当前改动存起来试试嘛喵~",
        );
        return;
    }
    let rows = app.stash_rows;
    let selected = app
        .stash_selected
        .min(app.repo.stashes.len().saturating_sub(1));
    let start = (selected / rows) * rows;
    let end = (start + rows).min(app.repo.stashes.len());
    let subject_width = body.width.saturating_sub(12) as usize;
    for (offset, index) in (start..end).enumerate() {
        let stash = &app.repo.stashes[index];
        let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
        if app.pane == LocalPane::Stashes && index == selected {
            fill(frame, row, theme.color("surface1"));
        } else if app.hover_pane == Some(LocalPane::Stashes) && app.hover_row == Some(index) {
            fill(frame, row, theme.color("surface0"));
        }
        let mut spans = vec![
            Span::styled(
                format!("{} ", icons::ARCHIVE),
                Style::default().fg(theme.color("peach")),
            ),
            Span::styled(
                format!("{} ", stash.name),
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        if subject_width >= 4 {
            spans.push(Span::styled(
                format::truncate(&format::clean_text(&stash.subject), subject_width),
                Style::default().fg(theme.color("text")),
            ));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), row);
    }
    scrollbar(frame, theme, body, app.repo.stashes.len(), rows, start);
}

fn draw_help(frame: &mut Frame, theme: &Theme, area: Rect) {
    let width = area.width.saturating_sub(8).min(84);
    let height = area.height.saturating_sub(2).min(24);
    if width < 50 || height < 12 {
        return;
    }
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.color("blue")))
        .title(Line::from(Span::styled(
            " 本地仓库帮助喵~ ",
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD),
        )));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    fill(frame, inner, theme.color("mantle"));
    let lines = [
        ("Tab / 1~6", "切换面板"),
        ("j k / ↑ ↓", "选择；[ ] 或 PgUp/PgDn 翻页"),
        ("g / G", "第一项 / 最后一项"),
        ("Enter", "分支：切换；stash：应用；其他：刷新差异"),
        ("s / u", "暂存 / 取消暂存；S / U 全部"),
        ("d", "在暂存和未暂存差异之间切换"),
        ("c / A", "提交 / 修改最后一条提交"),
        ("t / b", "创建标签 / 创建并切换分支"),
        ("M / R / X", "分支：合并 / 变基 / 删除（需确认）"),
        ("Q", "中止进行中的合并或变基"),
        ("z / Z", "保存 stash / 恢复最近一条 stash"),
        ("a / D", "Stash：应用 / 删除选中 stash"),
        ("x", "丢弃选中文件的全部改动（需确认）"),
        ("p / P", "推送当前分支 / 推送标签"),
        ("f / L", "fetch / pull（仅 fast-forward）"),
        ("H / r", "查看文件提交历史 / 刷新仓库"),
        ("h / ?   q / Esc", "帮助 / 退出；帮助打开时先关帮助"),
    ];
    let limit = inner.height as usize;
    for (index, (keys, text)) in lines.iter().take(limit).enumerate() {
        let y = inner.y + index as u16;
        let pad = 16usize.saturating_sub(format::text_width(keys));
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{keys}{}", " ".repeat(pad)),
                    Style::default()
                        .fg(theme.color("blue"))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(*text, Style::default().fg(theme.color("subtext0"))),
            ])),
            Rect::new(inner.x, y, inner.width, 1),
        );
    }
}

fn draw_footer(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    fill(frame, area, theme.color("crust"));
    let (icon, color) = if app.busy.is_some() {
        (icons::REFRESH, theme.color("peach"))
    } else if app.message_error {
        (icons::WARNING, theme.color("red"))
    } else {
        (icons::INFO, theme.color("subtext0"))
    };
    let message = app.busy.as_deref().unwrap_or(&app.message);
    let frame_no = (app.started_at.elapsed().as_millis() / 120) as u64;
    let status = if app.busy.is_some() {
        format!("{}  {message}", format::spinner(frame_no))
    } else {
        format!("{icon}  {message}")
    };
    let hint = if area.width >= 100 && app.busy.is_none() {
        hover_hint(app)
    } else {
        None
    };
    let hint_width = hint
        .as_deref()
        .map(|text| format::text_width(text) as u16 + 2)
        .unwrap_or(0);
    let status_width = area.width.saturating_sub(4 + hint_width);
    frame.render_widget(
        Paragraph::new(format::truncate(&status, status_width as usize))
            .style(Style::default().fg(color)),
        Rect::new(area.x + 2, area.y, status_width, 1),
    );
    if let Some(hint) = hint {
        frame.render_widget(
            Paragraph::new(hint)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1),
        );
    }
    let mut x = area.x + 2;
    let end = area.x + area.width.saturating_sub(2);
    let switch = "Tab 切面板  ";
    let switch_width = format::text_width(switch) as u16;
    frame.render_widget(
        Paragraph::new(switch).style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(x, area.y + 1, switch_width, 1),
    );
    x += switch_width;
    for (action, key, label) in pane_hints(app.pane) {
        let hovered = app.hover == Some(*action);
        let text = format!(" {key} {label} ");
        let width = format::text_width(&text) as u16;
        if x + width > end {
            break;
        }
        let rect = Rect::new(x, area.y + 1, width, 1);
        if hovered {
            fill(frame, rect, theme.color("surface0"));
        }
        let style = if hovered {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("overlay0"))
        };
        frame.render_widget(Paragraph::new(text).style(style), rect);
        app.hits.push((rect, *action));
        x += width;
    }
}

fn hover_hint(app: &LocalApp) -> Option<String> {
    let row = app.hover_row?;
    let text = match app.hover_pane? {
        LocalPane::Status => {
            let file = app.repo.files.get(row)?;
            format!(
                "{}  ·  s 暂存  u 取消  x 丢弃",
                format::truncate(&format::clean_text(&file.path), 32)
            )
        }
        LocalPane::Commits => {
            let commit = app.repo.commits.get(row)?;
            format!(
                "{}  ·  {}",
                commit.short,
                format::truncate(&format::clean_text(&commit.summary), 36)
            )
        }
        LocalPane::Branches => {
            let branch = app.repo.branches.get(row)?;
            format!(
                "分支 {}  ·  Enter 切换  M 合并  R 变基  X 删除",
                format::clean_text(&branch.name)
            )
        }
        LocalPane::Tags => {
            let tag = app.repo.tags.get(row)?;
            format!("标签 {}  ·  P 推送标签", format::clean_text(&tag.name))
        }
        LocalPane::Stashes => {
            let stash = app.repo.stashes.get(row)?;
            format!(
                "{}  ·  a 应用  D 删除",
                format::truncate(&format::clean_text(&stash.subject), 36)
            )
        }
        LocalPane::Diff => return None,
    };
    Some(format::truncate(&text, 72))
}

fn pane_hints(pane: LocalPane) -> &'static [(LocalAction, &'static str, &'static str)] {
    match pane {
        LocalPane::Status => &[
            (LocalAction::Stage, "s", "暂存"),
            (LocalAction::Unstage, "u", "取消"),
            (LocalAction::Commit, "c", "提交"),
            (LocalAction::Push, "p", "推送"),
            (LocalAction::Fetch, "f", "取回"),
            (LocalAction::Pull, "L", "拉取"),
            (LocalAction::Refresh, "r", "刷新"),
            (LocalAction::Help, "h", "帮助"),
            (LocalAction::Quit, "q", "退出"),
            (LocalAction::Discard, "x", "丢弃"),
            (LocalAction::StashSave, "z", "stash"),
        ],
        LocalPane::Commits => &[
            (LocalAction::Commit, "c", "提交"),
            (LocalAction::Amend, "A", "修改"),
            (LocalAction::Push, "p", "推送"),
            (LocalAction::Fetch, "f", "取回"),
            (LocalAction::Pull, "L", "拉取"),
            (LocalAction::Refresh, "r", "刷新"),
            (LocalAction::Help, "h", "帮助"),
            (LocalAction::Quit, "q", "退出"),
        ],
        LocalPane::Branches => &[
            (LocalAction::CheckoutBranch, "Enter", "切换"),
            (LocalAction::Merge, "M", "合并"),
            (LocalAction::Rebase, "R", "变基"),
            (LocalAction::DeleteBranch, "X", "删除"),
            (LocalAction::NewBranch, "b", "新建"),
            (LocalAction::Abort, "Q", "中止"),
            (LocalAction::Refresh, "r", "刷新"),
            (LocalAction::Help, "h", "帮助"),
            (LocalAction::Quit, "q", "退出"),
        ],
        LocalPane::Tags => &[
            (LocalAction::Tag, "t", "新标签"),
            (LocalAction::PushTags, "P", "推标签"),
            (LocalAction::Refresh, "r", "刷新"),
            (LocalAction::Help, "h", "帮助"),
            (LocalAction::Quit, "q", "退出"),
        ],
        LocalPane::Stashes => &[
            (LocalAction::StashSave, "z", "保存"),
            (LocalAction::StashPop, "Z", "弹出"),
            (LocalAction::StashApply, "a", "应用"),
            (LocalAction::StashDrop, "D", "删除"),
            (LocalAction::Refresh, "r", "刷新"),
            (LocalAction::Help, "h", "帮助"),
            (LocalAction::Quit, "q", "退出"),
        ],
        LocalPane::Diff => &[
            (LocalAction::Stage, "s", "暂存"),
            (LocalAction::Unstage, "u", "取消"),
            (LocalAction::Refresh, "r", "刷新"),
            (LocalAction::Help, "h", "帮助"),
            (LocalAction::Quit, "q", "退出"),
        ],
    }
}

fn draw_prompt(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    let Some(prompt) = app.prompt.as_ref() else {
        return;
    };
    let (title, placeholder) = match prompt.kind {
        PromptKind::Commit => ("提交信息喵~", "写点什么喵~"),
        PromptKind::Amend => ("修改提交信息喵~", "新的提交信息喵~"),
        PromptKind::Tag => ("标签名喵~", "比如 v0.1.2 喵~"),
        PromptKind::Branch => ("新分支名喵~", "比如 feature/xxx 喵~"),
        PromptKind::Discard => ("确认丢弃喵~", ""),
        PromptKind::StashDrop => ("确认删除 stash喵~", ""),
        PromptKind::Merge => ("确认合并喵~", ""),
        PromptKind::Rebase => ("确认变基喵~", ""),
        PromptKind::BranchDelete => ("确认删除分支喵~", ""),
        PromptKind::Abort => ("确认中止喵~", ""),
    };
    let width = area.width.saturating_sub(12).min(70);
    let height = 8.min(area.height.saturating_sub(4));
    if width < 36 || height < 6 {
        return;
    }
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.color("blue")))
        .title(Line::from(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD),
        )));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    fill(frame, inner, theme.color("mantle"));
    if prompt.kind.is_confirm() {
        let target = prompt.target.as_deref().unwrap_or("");
        let (target, warning, confirm) = match prompt.kind {
            PromptKind::StashDrop => (
                target.to_string(),
                "这条 stash 会被真实删除喵~",
                "Enter 确认删除   Esc 取消喵~",
            ),
            PromptKind::Merge => (
                format!("{} → {}", target, app.repo.branch),
                "选中分支会合进当前分支喵~",
                "Enter 确认合并   Esc 取消喵~",
            ),
            PromptKind::Rebase => (
                format!("{} → {}", app.repo.branch, target),
                "当前分支会变基到选中分支上喵~",
                "Enter 确认变基   Esc 取消喵~",
            ),
            PromptKind::BranchDelete => (
                target.to_string(),
                "这条分支会被真实删除（未合并会失败）喵~",
                "Enter 确认删除   Esc 取消喵~",
            ),
            PromptKind::Abort => (
                "当前进行中的合并或变基".to_string(),
                "已解决的冲突进度会丢失喵~",
                "Enter 确认中止   Esc 取消喵~",
            ),
            _ => (
                target.to_string(),
                "这个文件的全部改动都会被丢弃喵~",
                "Enter 确认丢弃   Esc 取消喵~",
            ),
        };
        frame.render_widget(
            Paragraph::new(format::truncate(&target, inner.width as usize))
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(inner.x, inner.y + 1, inner.width, 1),
        );
        frame.render_widget(
            Paragraph::new(warning).style(Style::default().fg(theme.color("red"))),
            Rect::new(inner.x, inner.y + 2, inner.width, 1),
        );
        frame.render_widget(
            Paragraph::new(confirm).style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, inner.y + 4, inner.width, 1),
        );
        return;
    }
    let input_rect = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    fill(frame, input_rect, theme.color("surface0"));
    let (shown, cursor_column) = prompt.input.visible_window(inner.width as usize);
    let empty = prompt.input.is_empty();
    let shown = if empty {
        placeholder.to_string()
    } else {
        shown
    };
    frame.render_widget(
        Paragraph::new(shown).style(if empty {
            Style::default().fg(theme.color("overlay0"))
        } else {
            Style::default().fg(theme.color("text"))
        }),
        input_rect,
    );
    frame.set_cursor_position(Position::new(
        inner.x + cursor_column.min(inner.width.saturating_sub(1)),
        inner.y + 1,
    ));
    frame.render_widget(
        Paragraph::new("Enter 确认   Esc 取消喵~")
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
        area.x + 1,
        area.y + area.height.saturating_sub(height) / 2,
        area.width.saturating_sub(2),
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
