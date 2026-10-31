use crate::format;
use crate::icons;
use crate::local::StatusFile;
use crate::local_app::{LocalAction, LocalApp, LocalLayout, LocalPane, PromptKind};
use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

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
    let footer_height = if area.width < 100 { 3 } else { 2 };
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(footer_height),
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
    branch.push(Span::styled(
        format!(
            "   {} 暂存 · {} 未暂存 · {} 提交 · {} 标签 · {} stash",
            app.repo.staged_count(),
            app.repo.unstaged_count(),
            app.repo.commits.len(),
            app.repo.tags.len(),
            app.repo.stashes.len()
        ),
        Style::default().fg(theme.color("overlay0")),
    ));
    if let Some(busy) = app.busy.as_deref() {
        let frame_no = (app.started_at.elapsed().as_millis() / 120) as u64;
        branch.push(Span::styled(
            format!("   {} {busy}", format::spinner(frame_no)),
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
    let commit_height = (area.height / 3).clamp(5, 9);
    let chunks = Layout::vertical([
        Constraint::Min(10),
        Constraint::Length(1),
        Constraint::Length(commit_height),
    ])
    .split(area);
    let main = chunks[0];
    let columns = Layout::horizontal([
        Constraint::Length(24),
        Constraint::Length(1),
        Constraint::Percentage(40),
        Constraint::Length(1),
        Constraint::Min(24),
    ])
    .split(main);
    let sidebar = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Length(1),
        Constraint::Min(5),
    ])
    .split(columns[0]);
    draw_branches(frame, app, theme, sidebar[0]);
    draw_tags(frame, app, theme, sidebar[2]);
    draw_status(frame, app, theme, columns[2]);
    draw_diff(frame, app, theme, columns[4]);
    let bottom = Layout::horizontal([
        Constraint::Percentage(72),
        Constraint::Length(1),
        Constraint::Min(20),
    ])
    .split(chunks[2]);
    draw_commits(frame, app, theme, bottom[0]);
    draw_stashes(frame, app, theme, bottom[2]);
}

fn panel(frame: &mut Frame, theme: &Theme, area: Rect, title: &str, focused: bool) -> Rect {
    let border = if focused {
        theme.color("blue")
    } else {
        theme.color("surface1")
    };
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title(Line::from(Span::styled(
            format!(" {title} "),
            if focused {
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.color("subtext0"))
            },
        )));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    fill(frame, inner, theme.color("mantle"));
    inner
}

fn page_label(total: usize, selected: usize, rows: usize) -> String {
    if total == 0 || rows == 0 {
        return String::new();
    }
    let pages = total.div_ceil(rows);
    let page = (selected / rows + 1).min(pages);
    format!("  {page}/{pages}")
}

fn draw_branches(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    let title = format!(
        "{} 分支喵~{}",
        icons::BRANCH,
        page_label(
            app.repo.branches.len(),
            app.branch_selected,
            app.branch_rows
        )
    );
    let body = panel(frame, theme, area, &title, app.pane == LocalPane::Branches);
    app.layout.branches = body;
    app.branch_rows = body.height.max(1) as usize;
    if body.height == 0 {
        return;
    }
    if app.repo.branches.is_empty() {
        draw_center(frame, theme, body, "还没有分支喵~", "先提交一次就有啦喵~");
        return;
    }
    let rows = app.branch_rows;
    let selected = app
        .branch_selected
        .min(app.repo.branches.len().saturating_sub(1));
    let start = (selected / rows) * rows;
    let end = (start + rows).min(app.repo.branches.len());
    for (offset, index) in (start..end).enumerate() {
        let branch = &app.repo.branches[index];
        let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
        if app.pane == LocalPane::Branches && index == selected {
            fill(frame, row, theme.color("surface0"));
        }
        let marker = if branch.current { "*" } else { " " };
        let line = Line::from(vec![
            Span::styled(marker, Style::default().fg(theme.color("green"))),
            Span::styled(
                format!(" {}", format::clean_text(&branch.name)),
                if branch.current {
                    Style::default()
                        .fg(theme.color("blue"))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.color("text"))
                },
            ),
            Span::styled(
                if branch.upstream.is_empty() {
                    String::new()
                } else {
                    format!("  {}", branch.upstream)
                },
                Style::default().fg(theme.color("overlay0")),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }
}

fn draw_tags(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    let title = format!(
        "{} 标签喵~{}",
        icons::TAG,
        page_label(app.repo.tags.len(), app.tag_selected, app.tag_rows)
    );
    let body = panel(frame, theme, area, &title, app.pane == LocalPane::Tags);
    app.layout.tags = body;
    app.tag_rows = body.height.max(1) as usize;
    if body.height == 0 {
        return;
    }
    if app.repo.tags.is_empty() {
        draw_center(frame, theme, body, "还没有标签喵~", "按 t 建一个试试嘛喵~");
        return;
    }
    let rows = app.tag_rows;
    let selected = app.tag_selected.min(app.repo.tags.len().saturating_sub(1));
    let start = (selected / rows) * rows;
    let end = (start + rows).min(app.repo.tags.len());
    for (offset, index) in (start..end).enumerate() {
        let tag = &app.repo.tags[index];
        let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
        if app.pane == LocalPane::Tags && index == selected {
            fill(frame, row, theme.color("surface0"));
        }
        let line = Line::from(vec![
            Span::styled(
                format!("{} ", icons::TAG),
                Style::default().fg(theme.color("peach")),
            ),
            Span::styled(
                format::clean_text(&tag.name),
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", tag.when),
                Style::default().fg(theme.color("overlay0")),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }
}

fn draw_status(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    let title = format!(
        "{} 改动喵~  暂存 {} · 未暂存 {}{}",
        icons::FILE,
        app.repo.staged_count(),
        app.repo.unstaged_count(),
        page_label(app.repo.files.len(), app.file_selected, app.status_rows)
    );
    let body = panel(frame, theme, area, &title, app.pane == LocalPane::Status);
    app.layout.status = body;
    app.status_rows = body.height.max(1) as usize;
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
    let rows = app.status_rows;
    let selected = app
        .file_selected
        .min(app.repo.files.len().saturating_sub(1));
    let start = (selected / rows) * rows;
    let end = (start + rows).min(app.repo.files.len());
    for (offset, index) in (start..end).enumerate() {
        let file = &app.repo.files[index];
        let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
        if app.pane == LocalPane::Status && index == selected {
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
                format::clean_text(&file.path),
                Style::default().fg(theme.color("text")),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }
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
    let rows = area.height.saturating_sub(2) as usize;
    let title = format!(
        "{}{}",
        app.diff_title,
        if rows > 0 {
            let lines = app.diff_text.lines().count();
            if lines > rows {
                let page = app.diff_scroll as usize / rows + 1;
                let pages = lines.div_ceil(rows);
                format!("  {page}/{pages}")
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    );
    let body = panel(frame, theme, area, &title, app.pane == LocalPane::Diff);
    app.layout.diff = body;
    app.diff_scroll_max = app
        .diff_text
        .lines()
        .count()
        .saturating_sub(body.height as usize) as u16;
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
    let title = format!(
        "{} 提交喵~{}",
        icons::CLOCK,
        page_label(app.repo.commits.len(), app.commit_selected, app.commit_rows)
    );
    let body = panel(frame, theme, area, &title, app.pane == LocalPane::Commits);
    app.layout.commits = body;
    app.commit_rows = (body.height / 2).max(1) as usize;
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
}

fn draw_stashes(frame: &mut Frame, app: &mut LocalApp, theme: &Theme, area: Rect) {
    let title = format!(
        "{} stash喵~{}",
        icons::ARCHIVE,
        page_label(app.repo.stashes.len(), app.stash_selected, app.stash_rows)
    );
    let body = panel(frame, theme, area, &title, app.pane == LocalPane::Stashes);
    app.layout.stashes = body;
    app.stash_rows = body.height.max(1) as usize;
    if body.height == 0 {
        return;
    }
    if app.repo.stashes.is_empty() {
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
    for (offset, index) in (start..end).enumerate() {
        let stash = &app.repo.stashes[index];
        let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
        if app.pane == LocalPane::Stashes && index == selected {
            fill(frame, row, theme.color("surface0"));
        }
        let subject_width = body.width.saturating_sub(18) as usize;
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
        spans.push(Span::styled(
            format!("  {}", stash.when),
            Style::default().fg(theme.color("overlay0")),
        ));
        frame.render_widget(Paragraph::new(Line::from(spans)), row);
    }
}

fn draw_help(frame: &mut Frame, theme: &Theme, area: Rect) {
    let width = area.width.saturating_sub(8).min(78);
    let height = area.height.saturating_sub(4).min(22);
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.color("green")))
        .title(Line::from(Span::styled(
            " 本地仓库帮助喵~ ",
            Style::default()
                .fg(theme.color("green"))
                .add_modifier(Modifier::BOLD),
        )));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    fill(frame, inner, theme.color("mantle"));
    let lines = [
        ("Tab / ← →", "切换面板"),
        ("j k / ↑ ↓", "选择；[ ] 或 PgUp/PgDn 翻页"),
        ("g / G", "第一项 / 最后一项"),
        ("Enter", "切换分支；Stash 面板应用 stash；其他刷新差异"),
        ("s / u", "暂存 / 取消暂存；S / U 全部暂存 / 取消"),
        ("d", "在暂存和未暂存差异之间切换"),
        ("c / A", "提交 / 修改最后一条提交"),
        ("t / b", "创建标签 / 创建并切换分支"),
        ("z / Z", "保存 stash / 恢复最近一条 stash"),
        ("a / D", "Stash 面板：应用 / 删除选中 stash"),
        ("x", "丢弃选中文件的全部改动（需确认）"),
        ("p / P", "推送当前分支 / 推送标签"),
        ("f / L", "fetch / pull（仅 fast-forward）"),
        ("H", "查看选中文件的提交历史"),
        ("r / h / ?", "刷新仓库 / 打开或关闭帮助"),
        ("q / Esc", "退出人家；帮助打开时先关帮助"),
    ];
    let limit = inner.height as usize;
    for (index, (keys, text)) in lines.iter().take(limit).enumerate() {
        let y = inner.y + index as u16;
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{keys:<12}"),
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
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("{icon}  {message}"),
            area.width.saturating_sub(4) as usize,
        ))
        .style(Style::default().fg(color)),
        Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1),
    );
    let keys = [
        (LocalAction::Stage, "s", "暂存"),
        (LocalAction::Unstage, "u", "取消"),
        (LocalAction::Commit, "c", "提交"),
        (LocalAction::StashSave, "z", "stash"),
        (LocalAction::Push, "p", "推送"),
        (LocalAction::Fetch, "f", "取回"),
        (LocalAction::Pull, "L", "拉取"),
        (LocalAction::Refresh, "r", "刷新"),
        (LocalAction::Help, "h", "帮助"),
        (LocalAction::Quit, "q", "退出"),
    ];
    let mut x = area.x + 2;
    let mut y = area.y + 1;
    let end = area.x + area.width.saturating_sub(2);
    for (action, key, label) in keys {
        let hovered = app.hover == Some(action);
        let text = if hovered {
            format!(">{key} {label} ")
        } else {
            format!(" {key} {label} ")
        };
        let width = format::text_width(&text) as u16;
        if x + width > end {
            y += 1;
            x = area.x + 2;
            if y >= area.y + area.height {
                break;
            }
        }
        let rect = Rect::new(x, y, width, 1);
        if hovered {
            fill(frame, rect, theme.color("surface0"));
        }
        let style = if hovered {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("subtext0"))
        };
        frame.render_widget(Paragraph::new(text).style(style), rect);
        app.hits.push((rect, action));
        x += width;
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.color("green")))
        .title(Line::from(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(theme.color("green"))
                .add_modifier(Modifier::BOLD),
        )));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    fill(frame, inner, theme.color("mantle"));
    if matches!(prompt.kind, PromptKind::Discard | PromptKind::StashDrop) {
        let target = prompt.target.as_deref().unwrap_or("");
        let (warning, confirm) = if prompt.kind == PromptKind::StashDrop {
            ("这条 stash 会被真实删除喵~", "Enter 确认删除   Esc 取消喵~")
        } else {
            (
                "这个文件的全部改动都会被丢弃喵~",
                "Enter 确认丢弃   Esc 取消喵~",
            )
        };
        frame.render_widget(
            Paragraph::new(format::truncate(target, inner.width as usize))
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
