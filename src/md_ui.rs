use crate::format;
use crate::icons;
use crate::md_app::{Hit, MdApp, StatusKind, Tab};
use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

const MIN_WIDTH: u16 = 72;
const MIN_HEIGHT: u16 = 20;

pub fn draw(frame: &mut Frame, app: &mut MdApp) {
    let area = frame.area();
    let theme = app.theme.clone();
    fill(frame, area, theme.color("base"));

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        app.hits.clear();
        draw_too_small(frame, area, &theme);
        return;
    }

    app.hits.clear();
    app.files_rect = Rect::default();
    app.outline_rect = Rect::default();
    app.preview_rect = Rect::default();

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .split(area);
    draw_header(frame, app, &theme, chunks[0]);

    if app.fullscreen {
        draw_preview(frame, app, &theme, chunks[1]);
    } else {
        let columns = Layout::horizontal([
            Constraint::Percentage(34),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(chunks[1]);
        if app.pane == Tab::Outline {
            draw_outline(frame, app, &theme, columns[0]);
        } else {
            draw_files(frame, app, &theme, columns[0]);
        }
        let left_focused = app.focus != Tab::Preview;
        draw_divider(frame, &theme, columns[1], left_focused);
        draw_preview(frame, app, &theme, columns[2]);
    }

    draw_footer(frame, app, &theme, chunks[2]);
    if app.show_help {
        app.hits.clear();
        draw_help(frame, app, &theme, area);
    }
}

fn draw_header(frame: &mut Frame, app: &MdApp, theme: &Theme, area: Rect) {
    fill(frame, area, theme.color("mantle"));
    let brand = Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1);
    let mut title = vec![
        Span::styled(
            format!("{} ", icons::BOOK),
            Style::default().fg(theme.color("blue")),
        ),
        Span::styled(
            "gxt",
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if area.width > 90 {
        title.push(Span::styled(
            "  当前目录的 Markdown 预览器喵~",
            Style::default().fg(theme.color("overlay0")),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(title)), brand);

    let root = format!("{} {}", icons::FOLDER_OPEN, format::display_path(&app.root));
    let root_width = format::text_width(&root) as u16 + 2;
    if area.width > 100 && root_width < area.width / 2 {
        frame.render_widget(
            Paragraph::new(root)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                area.x + area.width.saturating_sub(root_width),
                area.y,
                root_width,
                1,
            ),
        );
    }

    let tab = icons::label(icons::BOOK, "Markdown 预览喵~");
    let tab_width = format::text_width(&tab) as u16 + 4;
    let tab_rect = Rect::new(area.x + 2, area.y + 1, tab_width, 1);
    fill(frame, tab_rect, theme.color("surface1"));
    frame.render_widget(
        Paragraph::new(tab).alignment(Alignment::Center).style(
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD),
        ),
        tab_rect,
    );
    let status = format!(
        "{} 个文件 · {} 个标题",
        app.files.len(),
        app.doc.as_ref().map(|doc| doc.headings.len()).unwrap_or(0)
    );
    if area.width > tab_width + 40 {
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                area.x + tab_width + 4,
                area.y + 1,
                area.width.saturating_sub(tab_width + 6),
                1,
            ),
        );
    }
    let hint = "Ctrl+F 搜索   Tab 换焦点   / 筛选   t 主题   ? 帮助喵~";
    if area.width > 116 {
        frame.render_widget(
            Paragraph::new(hint)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
        );
    }
    fill(
        frame,
        Rect::new(area.x, area.y + 2, area.width, 1),
        theme.color("surface0"),
    );
}

fn draw_footer(frame: &mut Frame, app: &MdApp, theme: &Theme, area: Rect) {
    fill(frame, area, theme.color("crust"));
    let (icon, color) = match app.status_kind {
        StatusKind::Info => (icons::INFO, theme.color("subtext0")),
        StatusKind::Ok => (icons::CHECK, theme.color("green")),
        StatusKind::Error => (icons::WARNING, theme.color("red")),
    };
    let status = if app.status.is_empty() {
        format!("{icon}  打开一篇 Markdown 看看喵~")
    } else {
        format!("{icon}  {}", app.status)
    };
    frame.render_widget(
        Paragraph::new(format::truncate(
            &status,
            area.width.saturating_sub(4) as usize,
        ))
        .style(Style::default().fg(color)),
        Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1),
    );

    let hints = hints_for(app);
    let right = format!(
        "{} {} 文件 · {} {} · gxt {}",
        icons::BOOK,
        app.visible.len(),
        icons::PALETTE,
        app.theme.display_name(),
        env!("CARGO_PKG_VERSION")
    );
    let right_width = format::text_width(&right) as u16;
    let left_width = area
        .width
        .saturating_sub(right_width + 6)
        .min(format::text_width(hints) as u16);
    frame.render_widget(
        Paragraph::new(format::truncate(hints, left_width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 2, area.y + 1, left_width, 1),
    );
    if area.width > right_width + 8 {
        frame.render_widget(
            Paragraph::new(right)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
        );
    }
}

fn hints_for(app: &MdApp) -> &'static str {
    if app.search_editing {
        return "输入关键词  Enter 搜索  Esc 取消  Ctrl+U 清空喵~";
    }
    if app.filter_editing {
        return "输入关键词筛选   Enter 确认   Esc 取消   Ctrl+U 清空喵~";
    }
    if app.show_help {
        return "Esc / ? 关掉帮助喵~";
    }
    if !app.search_query.is_empty() {
        return "n/N 下一处/上一处  Esc 清搜索  Ctrl+F 重新搜索喵~";
    }
    if app.fullscreen {
        return "j/k 滚动  g/G 首尾  PgUp/PgDn 翻页  m 源码  F/Esc 退出全屏喵~";
    }
    match app.focus {
        Tab::Files => "j/k 挑  Enter 打开  / 筛选  s 排序  Tab 换焦点  r 刷新  q 退出喵~",
        Tab::Outline => "j/k 挑标题  Enter 跳到预览  Tab 换焦点  m 源码  F 全屏喵~",
        Tab::Preview => "j/k 滚动  g/G 首尾  PgUp/PgDn 翻页  m 源码  e 编辑  F 全屏  Tab 换焦点喵~",
    }
}

fn draw_divider(frame: &mut Frame, theme: &Theme, area: Rect, left_focused: bool) {
    fill(frame, area, theme.color("base"));
    let color = if left_focused {
        theme.color("surface2")
    } else {
        theme.color("surface1")
    };
    let mut row = 0;
    while row < area.height {
        frame.render_widget(
            Paragraph::new(Span::styled("┆", Style::default().fg(color))),
            Rect::new(area.x, area.y + row, 1, 1),
        );
        row += 2;
    }
}

fn draw_pane_tabs(frame: &mut Frame, app: &mut MdApp, theme: &Theme, area: Rect, active: Tab) {
    let files_label = format!("{} 文件 {}", icons::FILE, app.visible.len());
    let outline_label = format!(
        "{} 大纲 {}",
        icons::BOOK,
        app.doc.as_ref().map(|doc| doc.headings.len()).unwrap_or(0)
    );
    let mut x = area.x;
    for (tab, label) in [(Tab::Files, files_label), (Tab::Outline, outline_label)] {
        let width = format::text_width(&label) as u16 + 2;
        let rect = Rect::new(x, area.y, width, 1);
        let selected = tab == active;
        let hovered = app.hover == Some(Hit::FocusTab(tab));
        fill(
            frame,
            rect,
            if selected {
                theme.color("surface1")
            } else if hovered {
                theme.color("surface0")
            } else {
                theme.color("mantle")
            },
        );
        frame.render_widget(
            Paragraph::new(label).alignment(Alignment::Center).style(
                Style::default()
                    .fg(if selected {
                        theme.color("blue")
                    } else {
                        theme.color("subtext0")
                    })
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            rect,
        );
        register(app, rect, Hit::FocusTab(tab));
        x += width + 1;
    }
    let sort = format!("排序 {}", app.sort.label());
    if area.x + area.width > x + sort.len() as u16 + 2 {
        frame.render_widget(
            Paragraph::new(sort)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(x, area.y, area.x + area.width - x, 1),
        );
    }
}

fn draw_files(frame: &mut Frame, app: &mut MdApp, theme: &Theme, area: Rect) {
    app.files_rect = area;
    register(app, area, Hit::FilesPanel);
    fill(frame, area, theme.color("mantle"));
    draw_pane_tabs(
        frame,
        app,
        theme,
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(1), 1),
        Tab::Files,
    );
    fill(
        frame,
        Rect::new(area.x, area.y + 1, area.width, 1),
        theme.color("surface0"),
    );

    let filter_row = app.filter_editing || !app.filter.value.is_empty();
    let list_height = area
        .height
        .saturating_sub(2 + if filter_row { 1 } else { 0 });
    let body = Rect::new(
        area.x + 1,
        area.y + 2,
        area.width.saturating_sub(2),
        list_height,
    );
    if app.visible.is_empty() {
        let text = if app.files.is_empty() {
            "这里没有 Markdown 喵~"
        } else {
            "筛选后什么都不剩喵~"
        };
        frame.render_widget(
            Paragraph::new(text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(body.x, body.y + body.height / 2, body.width, 1),
        );
    } else {
        let viewport = body.height.max(1) as usize;
        if app.selected < app.offset {
            app.offset = app.selected;
        } else if app.selected >= app.offset + viewport {
            app.offset = app.selected + 1 - viewport;
        }
        app.offset = app.offset.min(app.visible.len().saturating_sub(1));
        let rows = app.visible.len().saturating_sub(app.offset).min(viewport);
        for row in 0..rows {
            let index = app.offset + row;
            let file = &app.files[app.visible[index]];
            let y = body.y + row as u16;
            let selected = index == app.selected;
            let hovered = app.hover == Some(Hit::FileRow(index));
            let current = app.doc.as_ref().is_some_and(|doc| doc.rel == file.rel);
            if selected || hovered {
                fill(
                    frame,
                    Rect::new(area.x, y, area.width, 1),
                    theme.color(if selected { "surface1" } else { "surface0" }),
                );
            } else if index % 2 == 1 {
                fill(
                    frame,
                    Rect::new(area.x, y, area.width, 1),
                    theme.color("base"),
                );
            }
            let icon = if current { icons::BOOK } else { icons::FILE };
            let icon_color = if current {
                theme.color("blue")
            } else {
                theme.color("overlay0")
            };
            let rail = if current {
                Span::styled("▌", Style::default().fg(theme.color("blue")))
            } else {
                Span::raw(" ")
            };
            let size = format::format_size(file.size);
            let room = body.width.saturating_sub(size.len() as u16 + 4) as usize;
            let rel = format::truncate(&file.rel, room.max(4));
            let pad = room.saturating_sub(format::text_width(&rel));
            let text_style = if selected {
                Style::default().fg(theme.color("text"))
            } else if current {
                Style::default().fg(theme.color("blue"))
            } else {
                Style::default().fg(theme.color("subtext0"))
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    rail,
                    Span::styled(format!("{icon} "), Style::default().fg(icon_color)),
                    Span::styled(rel, text_style),
                    Span::raw(" ".repeat(pad)),
                    Span::styled(size, Style::default().fg(theme.color("overlay0"))),
                ])),
                Rect::new(body.x, y, body.width, 1),
            );
            register(
                app,
                Rect::new(area.x, y, area.width, 1),
                Hit::FileRow(index),
            );
        }
    }

    if filter_row {
        let y = area.y + area.height.saturating_sub(1);
        let hovered = app.hover == Some(Hit::FilterRow);
        fill(
            frame,
            Rect::new(area.x, y, area.width, 1),
            theme.color(if hovered && !app.filter_editing {
                "surface1"
            } else {
                "surface0"
            }),
        );
        register(app, Rect::new(area.x, y, area.width, 1), Hit::FilterRow);
        let prefix = format!("{} ", icons::SEARCH);
        let prefix_width = format::text_width(&prefix) as u16;
        let max_width = area.width.saturating_sub(prefix_width + 3);
        let (text, cursor) = if app.filter_editing {
            app.filter.visible_window(max_width as usize)
        } else {
            (
                format::truncate(&app.filter.value, max_width.max(1) as usize),
                0,
            )
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(theme.color("blue"))),
                Span::styled(text, Style::default().fg(theme.color("text"))),
            ])),
            Rect::new(area.x + 1, y, area.width.saturating_sub(2), 1),
        );
        if app.filter_editing {
            frame.set_cursor_position(Position::new(area.x + 1 + prefix_width + cursor, y));
        }
    }
}
fn draw_outline(frame: &mut Frame, app: &mut MdApp, theme: &Theme, area: Rect) {
    app.outline_rect = area;
    register(app, area, Hit::OutlinePanel);
    fill(frame, area, theme.color("mantle"));
    draw_pane_tabs(
        frame,
        app,
        theme,
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(1), 1),
        Tab::Outline,
    );
    fill(
        frame,
        Rect::new(area.x, area.y + 1, area.width, 1),
        theme.color("surface0"),
    );

    let body = Rect::new(
        area.x + 1,
        area.y + 2,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    let headings: Vec<(usize, String)> = app
        .doc
        .as_ref()
        .map(|doc| {
            doc.headings
                .iter()
                .map(|heading| (heading.level, heading.title.clone()))
                .collect()
        })
        .unwrap_or_default();
    if headings.is_empty() {
        let text = if app.doc.is_some() {
            "这篇没有标题喵~"
        } else {
            "先挑一篇文档喵~"
        };
        frame.render_widget(
            Paragraph::new(text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(body.x, body.y + body.height / 2, body.width, 1),
        );
        return;
    }
    let viewport = body.height.max(1) as usize;
    if app.outline_selected < app.outline_offset {
        app.outline_offset = app.outline_selected;
    } else if app.outline_selected >= app.outline_offset + viewport {
        app.outline_offset = app.outline_selected + 1 - viewport;
    }
    app.outline_offset = app.outline_offset.min(headings.len().saturating_sub(1));
    let rows = headings
        .len()
        .saturating_sub(app.outline_offset)
        .min(viewport);
    for row in 0..rows {
        let index = app.outline_offset + row;
        let (level, title) = &headings[index];
        let level = *level;
        let y = body.y + row as u16;
        let selected = index == app.outline_selected;
        let hovered = app.hover == Some(Hit::OutlineRow(index));
        if selected || hovered {
            fill(
                frame,
                Rect::new(area.x, y, area.width, 1),
                theme.color(if selected { "surface1" } else { "surface0" }),
            );
        }
        let color = match level {
            1 => theme.color("blue"),
            2 => theme.color("mauve"),
            3 => theme.color("peach"),
            _ => theme.color("overlay0"),
        };
        let indent = "  ".repeat(level.saturating_sub(1).min(4));
        let room = body.width.saturating_sub(indent.len() as u16 + 4).max(4) as usize;
        let title = format::truncate(title, room);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(indent, Style::default()),
                Span::styled(
                    format!("{} ", if level <= 2 { "▸" } else { "·" }),
                    Style::default().fg(color),
                ),
                Span::styled(
                    title,
                    Style::default()
                        .fg(if selected { theme.color("text") } else { color })
                        .add_modifier(if level == 1 {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
            ])),
            Rect::new(body.x, y, body.width, 1),
        );
        register(
            app,
            Rect::new(area.x, y, area.width, 1),
            Hit::OutlineRow(index),
        );
    }
}

fn draw_preview(frame: &mut Frame, app: &mut MdApp, theme: &Theme, area: Rect) {
    app.preview_rect = area;
    register(app, area, Hit::PreviewPanel);
    fill(frame, area, theme.color("base"));

    let inner_width = area.width.saturating_sub(4) as usize;
    let search_row = app.search_editing || !app.search_query.is_empty();
    let meta = app.doc.as_ref().map(|doc| {
        let progress = if app.preview_scroll_max > 0 {
            (app.preview_scroll as u32 * 100 / app.preview_scroll_max as u32).min(100)
        } else {
            100
        };
        format!(
            "{} 行 · {} 字符 · {} 标题 · {} · {}%",
            doc.content.lines().count(),
            doc.content.chars().count(),
            doc.headings.len(),
            if app.raw { "源码" } else { "渲染" },
            progress
        )
    });
    let title = app
        .doc
        .as_ref()
        .map(|doc| format!("{} {}", icons::BOOK, doc.rel))
        .unwrap_or_else(|| format!("{} 挑一篇 Markdown 看看喵~", icons::BOOK));
    let meta_room = meta
        .as_ref()
        .filter(|_| area.width > 76)
        .map(|meta| format::text_width(meta) + 2)
        .unwrap_or(0);
    let title_room = inner_width.saturating_sub(meta_room);
    let title_line = match app.doc.as_ref() {
        Some(doc) => {
            let (dir, name) = doc
                .rel
                .rsplit_once('/')
                .map(|(dir, name)| (format!("{dir}/"), name))
                .unwrap_or((String::new(), doc.rel.as_str()));
            let name_width = format::text_width(name);
            let available = title_room.saturating_sub(2);
            let mut spans = vec![Span::styled(
                format!("{} ", icons::BOOK),
                Style::default().fg(theme.color("blue")),
            )];
            if name_width >= available {
                spans.push(Span::styled(
                    format::truncate(name, available.max(4)),
                    Style::default()
                        .fg(theme.color("blue"))
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                let dir_room = available.saturating_sub(name_width);
                let dir_text = if format::text_width(&dir) > dir_room {
                    format::truncate(&dir, dir_room)
                } else {
                    dir.clone()
                };
                if !dir_text.is_empty() {
                    spans.push(Span::styled(
                        dir_text,
                        Style::default().fg(theme.color("overlay0")),
                    ));
                }
                spans.push(Span::styled(
                    name.to_string(),
                    Style::default()
                        .fg(theme.color("blue"))
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Line::from(spans)
        }
        None => Line::from(format::truncate(&title, title_room.max(8))),
    };
    frame.render_widget(
        Paragraph::new(title_line),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );

    if let Some(meta) = meta
        && area.width > 76
    {
        frame.render_widget(
            Paragraph::new(meta)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
        );
    }
    fill(
        frame,
        Rect::new(area.x, area.y + 1, area.width, 1),
        theme.color("surface0"),
    );

    let body = Rect::new(
        area.x + 1,
        area.y + 2,
        area.width.saturating_sub(3),
        area.height
            .saturating_sub(2 + if search_row { 1 } else { 0 }),
    );
    if area.width < 6 || body.height == 0 {
        return;
    }
    let Some(doc) = app.doc.as_ref() else {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "左边挑一篇文档，Enter 打开喵~",
                    Style::default().fg(theme.color("subtext0")),
                )),
                Line::default(),
                Line::from(Span::styled(
                    "j/k 自动预览，Tab 切到预览再滚动喵~",
                    Style::default().fg(theme.color("overlay0")),
                )),
            ])
            .alignment(Alignment::Center),
            Rect::new(
                body.x,
                body.y + body.height / 2,
                body.width,
                3.min(body.height),
            ),
        );
        return;
    };
    let width = body.width as usize;
    let mut lines = if app.raw {
        raw_lines(&doc.content, theme)
    } else {
        crate::ui::markdown_lines(&doc.content, width, theme)
    };
    if !app.search_query.is_empty() {
        lines = highlight_lines(
            lines,
            &app.search_query,
            app.search_matches.get(app.search_index).copied(),
            theme,
        );
    }
    let height = body.height as usize;
    let max = lines.len().saturating_sub(height);
    app.preview_width = width;
    app.preview_viewport = body.height;
    app.preview_scroll_max = max.min(u16::MAX as usize) as u16;
    app.preview_scroll = app.preview_scroll.min(app.preview_scroll_max);
    let scroll = app.preview_scroll;
    frame.render_widget(Paragraph::new(Text::from(lines)).scroll((scroll, 0)), body);

    if max > 0 {
        let track = Rect::new(
            area.x + area.width.saturating_sub(1),
            body.y,
            1,
            body.height,
        );
        fill(frame, track, theme.color("surface0"));
        let thumb = ((body.height as usize * body.height as usize) / (max + height))
            .max(1)
            .min(body.height as usize) as u16;
        let span = body.height.saturating_sub(thumb);
        let position = if max > 0 {
            scroll as u32 * span as u32 / max as u32
        } else {
            0
        } as u16;
        fill(
            frame,
            Rect::new(track.x, track.y + position, 1, thumb),
            theme.color("overlay0"),
        );
    }

    if search_row {
        let y = area.y + area.height.saturating_sub(1);
        let hovered = app.hover == Some(Hit::SearchRow);
        fill(
            frame,
            Rect::new(area.x, y, area.width, 1),
            theme.color(if hovered && !app.search_editing {
                "surface1"
            } else {
                "surface0"
            }),
        );
        register(app, Rect::new(area.x, y, area.width, 1), Hit::SearchRow);
        let prefix = format!("{} ", icons::SEARCH);
        let prefix_width = format::text_width(&prefix) as u16;
        let counter = format!(
            "{}/{}",
            app.search_matches
                .get(app.search_index)
                .map(|_| app.search_index + 1)
                .unwrap_or(0),
            app.search_matches.len()
        );
        let counter_width = format::text_width(&counter) as u16 + 2;
        let max_width = area.width.saturating_sub(prefix_width + counter_width + 4);
        let (text, cursor) = if app.search_editing {
            app.search.visible_window(max_width as usize)
        } else {
            (
                format::truncate(&app.search_query, max_width.max(1) as usize),
                0,
            )
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(theme.color("yellow"))),
                Span::styled(text, Style::default().fg(theme.color("text"))),
            ])),
            Rect::new(area.x + 1, y, area.width.saturating_sub(2), 1),
        );
        frame.render_widget(
            Paragraph::new(counter)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                area.x + 1,
                y,
                area.width.saturating_sub(counter_width + 1),
                1,
            ),
        );
        if app.search_editing {
            frame.set_cursor_position(Position::new(area.x + 1 + prefix_width + cursor, y));
        }
    }
}

fn raw_lines(text: &str, theme: &Theme) -> Vec<Line<'static>> {
    text.lines()
        .enumerate()
        .map(|(index, line)| {
            Line::from(vec![
                Span::styled(
                    format!("{:>4} ", index + 1),
                    Style::default().fg(theme.color("surface2")),
                ),
                Span::styled(
                    line.to_string(),
                    Style::default().fg(theme.color("overlay1")),
                ),
            ])
        })
        .collect()
}

fn highlight_lines(
    lines: Vec<Line<'static>>,
    query: &str,
    current: Option<usize>,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let needle = query.to_lowercase();
    if needle.is_empty() {
        return lines;
    }
    lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let is_current = current == Some(index);
            let mut spans = Vec::new();
            for span in line.spans {
                let text = span.content.into_owned();
                let lower = text.to_lowercase();
                if lower.len() != text.len() || !lower.contains(&needle) {
                    spans.push(Span::styled(text, span.style));
                    continue;
                }
                let mut start = 0;
                for (position, _) in lower.match_indices(&needle) {
                    if position > start {
                        spans.push(Span::styled(text[start..position].to_string(), span.style));
                    }
                    let end = position + needle.len();
                    let style = if is_current {
                        span.style
                            .bg(theme.color("yellow"))
                            .fg(theme.color("base"))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        span.style
                            .bg(theme.color("surface2"))
                            .fg(theme.color("text"))
                    };
                    spans.push(Span::styled(text[position..end].to_string(), style));
                    start = end;
                }
                if start < text.len() {
                    spans.push(Span::styled(text[start..].to_string(), span.style));
                }
            }
            Line::from(spans)
        })
        .collect()
}

fn draw_help(frame: &mut Frame, app: &mut MdApp, theme: &Theme, area: Rect) {
    let width = area.width.saturating_sub(10).min(76);
    let height = area.height.saturating_sub(6).min(22);
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
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.color("blue"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    let lines = [
        (
            "换焦点喵~",
            "Tab 在文件 / 大纲 / 预览之间转，Esc 回文件列表喵~",
        ),
        ("挑文件喵~", "j/k 上下挑并自动预览，Enter 打开并进预览喵~"),
        (
            "读文档喵~",
            "j/k 滚动，PgUp/PgDn 翻页，g/G 首尾，m 切源码喵~",
        ),
        ("看大纲喵~", "Tab 到大纲，j/k 挑标题，Enter 跳到对应段落喵~"),
        (
            "搜索喵~",
            "Ctrl+F 搜当前文档，Enter 确认，n/N 下一处/上一处，Esc 清掉喵~",
        ),
        (
            "筛选喵~",
            "/ 输入关键词，Enter 确认，Esc 取消，Ctrl+U 清空喵~",
        ),
        ("排序喵~", "s 按路径 / 大小 / 时间排序喵~"),
        ("换主题喵~", "t 循环 Catppuccin 四套主题，并写进配置喵~"),
        ("全屏喵~", "F 只留预览，Esc 或 F 再按一次回来喵~"),
        ("编辑喵~", "e 用 $VISUAL 或 $EDITOR 打开当前文件喵~"),
        ("刷新喵~", "r 重新扫描当前目录和子目录喵~"),
        ("鼠标喵~", "点行选中，滚轮滚列表或预览喵~"),
        ("退出喵~", "q 或 Esc 退出喵~"),
    ];
    frame.render_widget(
        Paragraph::new(icons::label(icons::INFO, "Markdown 预览器喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    for (y, (label, text)) in (inner.y + 2..).zip(lines) {
        if y + 1 >= inner.y + inner.height {
            break;
        }
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{label:<8}"),
                    Style::default().fg(theme.color("blue")),
                ),
                Span::styled(text, Style::default().fg(theme.color("subtext0"))),
            ])),
            Rect::new(inner.x, y, inner.width, 1),
        );
    }
    let close = Rect::new(inner.x, inner.y + inner.height.saturating_sub(1), 10, 1);
    let close_hovered = app.hover == Some(Hit::CloseHelp);
    fill(
        frame,
        close,
        theme.color(if close_hovered {
            "surface2"
        } else {
            "surface1"
        }),
    );
    frame.render_widget(
        Paragraph::new("关掉喵~")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(if close_hovered {
                        theme.color("text")
                    } else {
                        theme.color("blue")
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        close,
    );
    register(app, close, Hit::CloseHelp);
}

fn draw_too_small(frame: &mut Frame, area: Rect, theme: &Theme) {
    fill(frame, area, theme.color("base"));
    let lines = vec![
        Line::from(Span::styled(
            "终端再大一点嘛喵~",
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("至少要 {MIN_WIDTH} x {MIN_HEIGHT}，现在有点挤喵~"),
            Style::default().fg(theme.color("overlay0")),
        )),
    ];
    let rect = Rect::new(
        area.x + 2,
        area.y + area.height.saturating_sub(4) / 2,
        area.width.saturating_sub(4),
        4.min(area.height),
    );
    frame.render_widget(
        Paragraph::new(Text::from(lines)).alignment(Alignment::Center),
        rect,
    );
}

fn register(app: &mut MdApp, rect: Rect, hit: Hit) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    app.hits.push((rect, hit));
}

fn fill(frame: &mut Frame, rect: Rect, color: Color) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    frame.render_widget(Block::new().style(Style::default().bg(color)), rect);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Paths;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn renders_file_list_and_rendered_preview() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("gxt-md-ui-{stamp}"));
        let config = std::env::temp_dir().join(format!("gxt-md-ui-cfg-{stamp}"));
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(
            root.join("docs/guide.md"),
            "# 标题喵~\n\n正文 **加粗** 和 `代码` 喵~\n\n## 二级\n",
        )
        .unwrap();
        let paths = Paths::for_test(config.clone(), config.join("cache"));
        let mut app = MdApp::new(paths, root.clone(), Some("mocha".to_string()));
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        // Wide CJK glyphs keep a trailing cell; compare without whitespace.
        let compact: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
        assert!(compact.contains("Markdown预览"));
        assert!(text.contains("docs/guide.md"));
        assert!(compact.contains("标题喵~"));
        assert!(compact.contains("正文加粗和代码喵"));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&config);
    }

    #[test]
    fn searches_current_document_and_cycles_matches() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("gxt-md-search-{stamp}"));
        let config = std::env::temp_dir().join(format!("gxt-md-search-cfg-{stamp}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("search.md"),
            "# 标题\n\nalpha beta\n\nalpha gamma\n",
        )
        .unwrap();
        let paths = Paths::for_test(config.clone(), config.join("cache"));
        let mut app = MdApp::new(paths, root.clone(), Some("mocha".to_string()));
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));
        for ch in "alpha".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.search_matches.len(), 2);
        assert_eq!(app.search_index, 0);

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect();
        assert!(text.contains("1/2"));
        assert!(text.contains("alpha"));

        app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        assert_eq!(app.search_index, 1);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.search_query.is_empty());

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&config);
    }

    #[test]
    fn hover_tracks_file_rows() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("gxt-md-hover-{stamp}"));
        let config = std::env::temp_dir().join(format!("gxt-md-hover-cfg-{stamp}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("hover.md"), "# 悬停\n\n鼠标喵~\n").unwrap();
        let paths = Paths::for_test(config.clone(), config.join("cache"));
        let mut app = MdApp::new(paths, root.clone(), Some("mocha".to_string()));
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let (rect, hit) = app
            .hits
            .iter()
            .copied()
            .find(|(_, hit)| matches!(hit, Hit::FileRow(0)))
            .unwrap();
        app.handle_mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: rect.x + 2,
            row: rect.y,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.hover, Some(hit));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&config);
    }
}
