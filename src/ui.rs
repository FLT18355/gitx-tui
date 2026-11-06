use crate::app::{
    Action, App, HitBox, IssueKind, LayoutRects, Overlay, Page, RepoPane, StatusKind, TextInput,
    TtlKind, cache_groups,
};
use crate::format;
use crate::icons;
use crate::model::{Issue, IssueState, Repo, RepoSort, SearchSort, StarSort, TrendingPeriod};
use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

const MIN_WIDTH: u16 = 72;
const MIN_HEIGHT: u16 = 22;
const CARD_HEIGHT: u16 = 4;
const CARD_STEP: u16 = 5;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let theme = app.theme.clone();
    fill(frame, area, theme.color("base"));

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area, &theme);
        app.hitboxes.clear();
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .split(area);
    let mut hits = Vec::new();
    app.layout = LayoutRects::default();
    draw_header(frame, app, &theme, chunks[0], &mut hits);
    match app.page {
        Page::Home => draw_home_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Repositories => draw_repositories_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Starred => draw_starred_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Search => draw_search_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Trending => draw_trending_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Account => draw_account_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Profile => draw_profile_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Notifications => draw_notifications_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Settings => draw_settings_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Insights => draw_insights_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Repo => draw_repo_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Actions => draw_actions_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Issues => draw_issues_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Commits => draw_commits_page(frame, app, &theme, chunks[1], &mut hits),
        Page::ReleaseDetail => draw_release_page(frame, app, &theme, chunks[1], &mut hits),
    }
    draw_footer(frame, app, &theme, chunks[2], &hits);

    if app.overlay.is_some() {
        hits.clear();
    }
    match app.overlay {
        Some(Overlay::Help) => draw_help(frame, app, &theme, area, &mut hits),
        Some(Overlay::Languages) => draw_languages_overlay(frame, app, &theme, area, &mut hits),
        Some(Overlay::RepoLanguages) => {
            draw_repo_languages_overlay(frame, app, &theme, area, &mut hits)
        }
        Some(Overlay::NewIssue) => draw_new_issue_overlay(frame, app, &theme, area, &mut hits),
        Some(Overlay::EditRepo) => draw_repo_edit_overlay(frame, app, &theme, area, &mut hits),
        Some(Overlay::Cache) => draw_cache_overlay(frame, app, &theme, area, &mut hits),
        Some(Overlay::RunWorkflow) => draw_workflow_overlay(frame, app, &theme, area, &mut hits),
        Some(Overlay::Branches) => draw_branches_overlay(frame, app, &theme, area, &mut hits),
        Some(Overlay::ForkRepo) => draw_fork_overlay(frame, app, &theme, area, &mut hits),
        Some(Overlay::DeleteRepo) => draw_delete_overlay(frame, app, &theme, area, &mut hits),
        Some(Overlay::Palette) => draw_palette(frame, app, &theme, area, &mut hits),
        Some(Overlay::Themes) => draw_theme_studio(frame, app, &theme, area, &mut hits),
        None => app.layout.overlay = Rect::default(),
    }

    app.hitboxes = hits;
    colorize_icons(frame.buffer_mut(), &theme);
}

fn draw_header(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    let brand = Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1);
    let mut title = vec![Span::styled(
        format!("{} ", icons::GITHUB),
        Style::default().fg(theme.color("blue")),
    )];
    title.extend(
        gradient_segments(
            &[
                (
                    "gxt".to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                (" · github workspace".to_string(), Style::default()),
            ],
            theme.color("blue"),
            theme.color("mauve"),
        )
        .into_iter()
        .map(|(text, style)| Span::styled(text, style)),
    );
    if area.width > 96 {
        title.push(Span::styled(
            format!("   {} 终端里的 GitHub 工作台", icons::SPARKLE),
            Style::default().fg(theme.color("overlay0")),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(title)), brand);

    if app.settings_only {
        frame.render_widget(
            Paragraph::new("全局设置喵~  Ctrl+S 保存  Esc 退出喵~")
                .style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
        );
        frame.render_widget(
            Paragraph::new(gradient_line(
                area.width as usize,
                "─",
                theme.color("surface1"),
                theme.color("overlay0"),
            )),
            Rect::new(area.x, area.y + 2, area.width, 1),
        );
        return;
    }

    let unread = app.unread_notifications();
    let account = if unread > 0 {
        format!(
            "{} 账号 {}   {} {unread} 条未读   {} {}喵~",
            icons::USER,
            app.auth_label(),
            icons::BELL,
            icons::PALETTE,
            app.theme.display_name()
        )
    } else {
        format!(
            "{} 账号 {}   {} {}喵~",
            icons::USER,
            app.auth_label(),
            icons::PALETTE,
            app.theme.display_name()
        )
    };
    if area.width > 126 {
        let account_width = format::text_width(&account) as u16 + 2;
        let right = Rect::new(
            area.x + area.width.saturating_sub(account_width),
            area.y,
            account_width,
            1,
        );
        frame.render_widget(
            Paragraph::new(account).style(Style::default().fg(theme.color("subtext0"))),
            right,
        );
    }

    let active = app.page.nav_index().or_else(|| app.return_page.nav_index());
    let labels = [
        icons::label(icons::HOME, "首页喵~"),
        icons::label(icons::REPO, "仓库喵~"),
        icons::label(icons::STAR, "收藏喵~"),
        icons::label(icons::SEARCH, "搜索喵~"),
        icons::label(icons::USER, "账号喵~"),
        if unread > 0 {
            format!("{} 通知 {unread}喵~", icons::BELL)
        } else {
            icons::label(icons::BELL, "通知喵~")
        },
        icons::label(icons::FLAME, "热门喵~"),
    ];
    let pages = [
        Page::Home,
        Page::Repositories,
        Page::Starred,
        Page::Search,
        Page::Account,
        Page::Notifications,
        Page::Trending,
    ];
    let width_list: Vec<u16> = labels
        .iter()
        .map(|label| format::text_width(label) as u16 + 4)
        .collect();
    let total_width = width_list.iter().sum::<u16>() + labels.len().saturating_sub(1) as u16;
    let hint = "0 首页   Ctrl+K 命令   / 搜索   ? 帮助喵~";
    let hint_text_width = format::text_width(hint) as u16;
    let hint_width = if area.width > 124 && total_width + hint_text_width + 8 <= area.width {
        hint_text_width + 3
    } else {
        0
    };
    let strip_left = area.x + 2;
    let strip_right = area.x + area.width.saturating_sub(hint_width + 2);
    let overflow = total_width > strip_right.saturating_sub(strip_left);
    let arrow_width = 3_u16;
    let tabs_left = if overflow {
        strip_left + arrow_width + 1
    } else {
        strip_left
    };
    let tabs_right = if overflow {
        strip_right.saturating_sub(arrow_width + 1)
    } else {
        strip_right
    };
    let tabs_width = tabs_right.saturating_sub(tabs_left);
    let fit_from = |start: usize| -> usize {
        let mut used = 0_u16;
        let mut count = 0;
        for width in width_list.iter().skip(start) {
            let need = if count == 0 { *width } else { *width + 1 };
            if used.saturating_add(need) > tabs_width {
                break;
            }
            used += need;
            count += 1;
        }
        count
    };
    let capacity = fit_from(0).max(1);
    let max_start = labels.len().saturating_sub(capacity);
    let mut start = app.nav_offset.min(max_start);
    if overflow && fit_from(start) == 0 {
        start = start.saturating_sub(1);
    }
    if !overflow {
        start = 0;
    }
    app.nav_offset = start;

    if overflow {
        for (rect, action, enabled, icon) in [
            (
                Rect::new(strip_left, area.y + 1, arrow_width, 1),
                Action::NavLeft,
                start > 0,
                icons::CHEVRON_LEFT,
            ),
            (
                Rect::new(
                    strip_right.saturating_sub(arrow_width),
                    area.y + 1,
                    arrow_width,
                    1,
                ),
                Action::NavRight,
                start < max_start,
                icons::CHEVRON_RIGHT,
            ),
        ] {
            let hovered = app.hover.as_ref() == Some(&action);
            fill(
                frame,
                rect,
                if hovered && enabled {
                    theme.color("surface1")
                } else {
                    theme.color("surface0")
                },
            );
            frame.render_widget(
                Paragraph::new(icon)
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(if enabled {
                        theme.color("blue")
                    } else {
                        theme.color("surface2")
                    })),
                rect,
            );
            register(hits, rect, action);
        }
    }

    let mut x = tabs_left;
    let mut selected_rect = None;
    for (index, label) in labels.iter().enumerate().skip(start) {
        let width = width_list[index];
        if x.saturating_add(width) > tabs_right {
            break;
        }
        let rect = Rect::new(x, area.y + 1, width, 1);
        let selected = Some(index) == active;
        if selected {
            selected_rect = Some(rect);
        }
        let action = Action::Navigate(pages[index]);
        let hovered = app.hover.as_ref() == Some(&action);
        let bg = if selected {
            theme.color("surface1")
        } else if hovered {
            theme.color("surface0")
        } else {
            theme.color("mantle")
        };
        fill(frame, rect, bg);
        let style = if selected {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else if hovered {
            Style::default().fg(theme.color("text"))
        } else {
            Style::default().fg(theme.color("subtext0"))
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(format!(" {label} "), style)))
                .alignment(Alignment::Center),
            rect,
        );
        register(hits, rect, action);
        x += width + 1;
    }

    if hint_width > 0 {
        let width = format::text_width(hint) as u16;
        let rect = Rect::new(
            area.x + area.width.saturating_sub(width + 2),
            area.y + 1,
            width,
            1,
        );
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(theme.color("overlay0"))),
            rect,
        );
    }
    frame.render_widget(
        Paragraph::new(gradient_line(
            area.width as usize,
            "─",
            theme.color("surface1"),
            theme.color("overlay0"),
        )),
        Rect::new(area.x, area.y + 2, area.width, 1),
    );
    if let Some(rect) = selected_rect {
        frame.render_widget(
            Paragraph::new(Line::from(gradient_chars(
                rect.width as usize,
                "━",
                theme.color("blue"),
                theme.color("mauve"),
            ))),
            Rect::new(rect.x, area.y + 2, rect.width, 1),
        );
    }
}

fn draw_footer(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, _hits: &[HitBox]) {
    fill(frame, area, theme.color("crust"));
    let status_rect = Rect::new(area.x + 2, area.y, area.width.saturating_sub(26), 1);
    let (text, color) = if let Some(download) = app.download_summary() {
        (
            format!("{}  {}", app.spinner(), download),
            theme.color("peach"),
        )
    } else if let Some(busy) = app.busy_text() {
        (format!("{}  {}", app.spinner(), busy), theme.color("peach"))
    } else {
        let (icon, color) = match app.status.kind {
            StatusKind::Info => (icons::INFO, theme.color("subtext0")),
            StatusKind::Ok => (icons::CHECK, theme.color("green")),
            StatusKind::Error => (icons::WARNING, theme.color("red")),
        };
        (format!("{icon}  {}", app.status.text), color)
    };
    frame.render_widget(
        Paragraph::new(format::truncate(&text, status_rect.width as usize))
            .style(Style::default().fg(color)),
        status_rect,
    );
    let elapsed = app.started_at.elapsed().as_secs();
    let session = if elapsed >= 3600 {
        format!("{}h{:02}m", elapsed / 3600, (elapsed % 3600) / 60)
    } else {
        format!("{}m{:02}s", elapsed / 60, elapsed % 60)
    };
    let timer = format!("{} {session}  ", icons::CLOCK);
    let timer_width = format::text_width(&timer) as u16;
    if area.width > 60 {
        frame.render_widget(
            Paragraph::new(timer)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                area.x + area.width.saturating_sub(timer_width),
                area.y,
                timer_width,
                1,
            ),
        );
    }

    if area.width > 68 {
        let info = format!(
            "{} 缓存 {} 个文件 · {}   {} {}喵~",
            icons::DATABASE,
            app.cache_stats.files,
            format::format_size(app.cache_stats.bytes),
            icons::GLOBE,
            proxy_label(app)
        );
        let width = area.width.saturating_sub(2);
        let rect = Rect::new(area.x + 2, area.y + 1, width, 1);
        let hint = hints_for(app);
        let info_width = format::text_width(&info) as u16;
        let left_width = width.saturating_sub(info_width + 2);
        frame.render_widget(
            Paragraph::new(format::truncate(hint, left_width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(rect.x, rect.y, left_width, 1),
        );
        frame.render_widget(
            Paragraph::new(info)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(rect.x, rect.y, width, 1),
        );
    } else {
        let hint = hints_for(app);
        frame.render_widget(
            Paragraph::new(format::truncate(hint, area.width as usize - 4))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
        );
    }
}

fn hints_for(app: &App) -> &'static str {
    if app.readme_fullscreen && app.page == Page::Repo {
        return "j/k 滚动    g/G 首尾    PgUp/PgDn 翻页    F/Esc 退出全屏喵~";
    }
    match app.page {
        Page::Home => "j/k 挑置顶  Enter 打开  P 置顶仓库  E 随机探索  T 主题  Ctrl+K 命令喵~",
        Page::Insights => "r 刷新统计  E 随机探索  T 主题  Ctrl+K 命令  Esc 回仓库喵~",
        Page::Repositories => {
            "j/k 挑    [ ] 翻页    Enter 打开    / 筛选    e 编辑    f 收藏    l 语言喵~"
        }
        Page::Starred => {
            "j/k 挑    [ ] 翻页    Enter 打开    / 筛选    f 收藏    o 主页    r 刷新喵~"
        }
        Page::Search => "回车搜索  / 结果筛选  l 语言  j/k 挑  [ ] 翻页  Enter 打开喵~",
        Page::Trending => "j/k 挑  [ ] 翻页  p 换时段  / 筛选  l 语言  Enter 打开  r 刷新喵~",
        Page::Account => "r 刷新账号和额度    Enter 打开主页    Esc 回仓库喵~",
        Page::Profile => "j/k 挑仓库  [ ] 翻页  Enter 打开  o 去 GitHub  r 刷新  Esc 返回喵~",
        Page::Notifications => {
            "j/k 挑通知  Enter 打开  m 已读  M 全部已读  u 未读/全部  r 刷新  Esc 返回喵~"
        }
        Page::Settings => "j/k 选    h/l 调    Enter 操作    Ctrl+S 保存    Esc 离开喵~",
        Page::Repo => {
            "Esc 返回  Tab 换面板  P 置顶  E 随机  T 主题  b 分支  C 提交  F README  L 语言  a 工作流  i 议题喵~"
        }
        Page::Actions => {
            "j/k 挑运行  Enter 去浏览器  R/x 运行工作流  PgUp/PgDn 滚任务  r 刷新  Esc 返回喵~"
        }
        Page::Issues => "j/k 挑  Enter 打开  n 新建议题  v 换类型  s 换状态  r 刷新  Esc 返回喵~",
        Page::Commits => {
            "j/k 挑提交  Enter/o 浏览器打开  右侧看改动文件  b 切分支  r 刷新  Esc 返回喵~"
        }
        Page::ReleaseDetail => "j/k 挑文件  Enter 下载  o 去 GitHub  PgUp/PgDn 滚说明  Esc 返回喵~",
    }
}

fn draw_repositories_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let sidebar_width = if area.width >= 122 {
        24
    } else if area.width >= 96 {
        22
    } else if area.width >= 82 {
        19
    } else {
        0
    };
    let detail_width = if area.width >= 126 {
        44
    } else if area.width >= 104 {
        38
    } else if area.width >= 90 {
        34
    } else {
        0
    };
    let columns = Layout::horizontal([
        Constraint::Length(sidebar_width),
        Constraint::Min(28),
        Constraint::Length(detail_width),
    ])
    .split(area);
    if sidebar_width > 0 {
        draw_sidebar(frame, app, theme, columns[0], hits);
    }
    draw_repositories_main(frame, app, theme, columns[1], hits);
    if detail_width > 0 {
        draw_repository_detail(frame, app, theme, columns[2], hits);
        app.layout.detail = columns[2];
    }
}

fn draw_sidebar(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let width = area.width.saturating_sub(3);
    let mut y = area.y + 1;

    let login = app
        .user
        .as_ref()
        .map(|user| user.login.clone())
        .or_else(|| app.login.clone())
        .unwrap_or_else(|| "未登录喵~".to_string());
    frame.render_widget(
        Paragraph::new(icons::label(icons::USER, "当前账号喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 2, y, width, 1),
    );
    y += 1;
    frame.render_widget(
        Paragraph::new(format::truncate(&login, width as usize)).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 2, y, width, 1),
    );
    y += 1;
    let auth = if app.client.token_present() {
        "gh 已经连上啦喵~"
    } else {
        "先跑一下 gh auth login 哦喵~"
    };
    frame.render_widget(
        Paragraph::new(format::truncate(auth, width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 2, y, width, 1),
    );
    y += 2;

    let lang_header = Rect::new(area.x, y, area.width.saturating_sub(1), 1);
    let lang_hover = app.hover.as_ref() == Some(&Action::OpenLanguages);
    if lang_hover {
        fill(frame, lang_header, theme.color("surface0"));
    }
    frame.render_widget(
        Paragraph::new(icons::label(icons::CODE, "语言筛选喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 2, y, width, 1),
    );
    register(hits, lang_header, Action::OpenLanguages);
    y += 1;

    let total = app.repos.len();
    let all_active = app.language_filter.is_none();
    let all_rect = Rect::new(area.x, y, area.width.saturating_sub(1), 1);
    draw_sidebar_item(
        frame,
        theme,
        all_rect,
        "全部喵~",
        total,
        all_active,
        app.hover.as_ref() == Some(&Action::Language(None)),
    );
    register(hits, all_rect, Action::Language(None));
    y += 1;

    let counts = app.language_counts();
    let language_limit = area.y + area.height.saturating_sub(7);
    let mut shown = 0;
    for (language, count) in &counts {
        if y >= language_limit {
            break;
        }
        let active = app
            .language_filter
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case(language))
            .unwrap_or(false);
        let action = Action::Language(Some(language.clone()));
        let hovered = app.hover.as_ref() == Some(&action);
        let rect = Rect::new(area.x, y, area.width.saturating_sub(1), 1);
        draw_sidebar_item(frame, theme, rect, language, *count, active, hovered);
        register(hits, rect, action);
        shown += 1;
        y += 1;
    }
    if shown < counts.len() && y < area.y + area.height.saturating_sub(4) {
        let action = Action::OpenLanguages;
        let hovered = app.hover.as_ref() == Some(&action);
        let rect = Rect::new(area.x, y, area.width.saturating_sub(1), 1);
        if hovered {
            fill(frame, rect, theme.color("surface0"));
        }
        frame.render_widget(
            Paragraph::new(format!("查看全部 {} 种喵~", counts.len()))
                .style(Style::default().fg(theme.color("blue"))),
            Rect::new(area.x + 2, y, width, 1),
        );
        register(hits, rect, action);
    }

    let cache_top = area.y + area.height.saturating_sub(5);
    if cache_top > y {
        frame.render_widget(
            Paragraph::new(icons::label(icons::DATABASE, "本地缓存喵~"))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, cache_top, width, 1),
        );
        let stats = format!(
            "{} 个文件 · {}喵~",
            app.cache_stats.files,
            format::format_size(app.cache_stats.bytes)
        );
        frame.render_widget(
            Paragraph::new(format::truncate(&stats, width as usize))
                .style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(area.x + 2, cache_top + 1, width, 1),
        );
        frame.render_widget(
            Paragraph::new("到设置页就能管缓存啦喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, cache_top + 2, width, 1),
        );
    }
}

fn draw_sidebar_item(
    frame: &mut Frame,
    theme: &Theme,
    rect: Rect,
    label: &str,
    count: usize,
    active: bool,
    hovered: bool,
) {
    if active || hovered {
        fill(frame, rect, theme.color("surface0"));
    }
    if active {
        fill(
            frame,
            Rect::new(rect.x, rect.y, 1, rect.height),
            theme.color("blue"),
        );
    }
    let style = if active {
        Style::default()
            .fg(theme.color("blue"))
            .add_modifier(Modifier::BOLD)
    } else if hovered {
        Style::default().fg(theme.color("text"))
    } else {
        Style::default().fg(theme.color("subtext0"))
    };
    let count_text = count.to_string();
    let left = Rect::new(rect.x + 2, rect.y, rect.width.saturating_sub(4), 1);
    frame.render_widget(
        Paragraph::new(format::truncate(label, left.width as usize)).style(style),
        left,
    );
    frame.render_widget(
        Paragraph::new(count_text)
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            rect.x + 2,
            rect.y,
            rect.width.saturating_sub(4),
            rect.height,
        ),
    );
}

fn draw_repositories_main(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    draw_page_title(
        frame,
        app,
        theme,
        inner,
        &icons::label(icons::REPO, "人家的仓库喵~"),
        "人家把账号下的仓库都整理好啦喵~",
        "刷新一下喵~",
        Action::Refresh,
        hits,
    );

    let (public, private, languages, stars) = app.repo_stats();
    let stats = format!(
        "公开 {}   私有 {}   语言 {}   星标合计 {}喵~",
        public,
        private,
        languages,
        format::human_count(stars)
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&stats, inner.width as usize))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );

    let chips_y = inner.y + 3;
    draw_sort_chips(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, chips_y, inner.width, 1),
        &icons::label(icons::CLOCK, "排序喵~"),
        &[
            (
                RepoSort::Updated.label(),
                Action::SetRepoSort(RepoSort::Updated),
                app.repo_sort == RepoSort::Updated,
            ),
            (
                RepoSort::Stars.label(),
                Action::SetRepoSort(RepoSort::Stars),
                app.repo_sort == RepoSort::Stars,
            ),
            (
                RepoSort::Name.label(),
                Action::SetRepoSort(RepoSort::Name),
                app.repo_sort == RepoSort::Name,
            ),
        ],
    );

    draw_list_filter(
        frame,
        app,
        theme,
        Rect::new(inner.x, inner.y + 4, inner.width, 1),
        hits,
    );
    draw_list_pagination(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, inner.y + 5, inner.width, 1),
    );
    let list = Rect::new(
        inner.x,
        inner.y + 7,
        inner.width,
        inner.height.saturating_sub(7),
    );
    app.layout.list = list;
    draw_cards(frame, app, theme, list, hits, EmptyKind::Repositories);
}

fn draw_starred_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let summary_width = if area.width >= 118 { 25 } else { 0 };
    let detail_width = if area.width >= 112 {
        38
    } else if area.width >= 94 {
        33
    } else {
        0
    };
    let columns = Layout::horizontal([
        Constraint::Length(summary_width),
        Constraint::Min(28),
        Constraint::Length(detail_width),
    ])
    .split(area);
    if summary_width > 0 {
        draw_starred_summary(frame, app, theme, columns[0]);
    }
    draw_starred_main(frame, app, theme, columns[1], hits);
    if detail_width > 0 {
        draw_repository_detail(frame, app, theme, columns[2], hits);
        app.layout.detail = columns[2];
    }
}

fn draw_starred_summary(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::STAR, "星星小账本喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let (count, languages, stars) = app.starred_stats();
    frame.render_widget(
        Paragraph::new(count.to_string()).style(
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new("个仓库收着星星呢喵~").style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!("{} 语言  {}喵~", icons::CODE, languages))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 5, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{} 星标  {}喵~",
            icons::STAR,
            format::human_count(stars)
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 6, inner.width, 1),
    );

    let mut y = inner.y + 9;
    frame.render_widget(
        Paragraph::new(icons::label(icons::INFO, "人家的小提示喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 1;
    let note = [
        "收藏列表来自 GitHub喵~",
        "点仓库卡片看详情喵~",
        "双击或按 Enter 就进",
        "仓库页喵~",
    ];
    for line in note {
        frame.render_widget(
            Paragraph::new(format::truncate(line, inner.width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
}

fn draw_starred_main(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    draw_page_title(
        frame,
        app,
        theme,
        inner,
        &icons::label(icons::STAR, "星星收藏喵~"),
        "点过星星的仓库，都收在这里啦喵~",
        "刷新一下喵~",
        Action::Refresh,
        hits,
    );
    let (count, _, _) = app.starred_stats();
    frame.render_widget(
        Paragraph::new(format!("人家这里收着 {} 个仓库呢喵~", count))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );
    draw_sort_chips(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
        &icons::label(icons::CLOCK, "排序喵~"),
        &[
            (
                StarSort::Starred.label(),
                Action::SetStarSort(StarSort::Starred),
                app.star_sort == StarSort::Starred,
            ),
            (
                StarSort::Updated.label(),
                Action::SetStarSort(StarSort::Updated),
                app.star_sort == StarSort::Updated,
            ),
            (
                StarSort::Stars.label(),
                Action::SetStarSort(StarSort::Stars),
                app.star_sort == StarSort::Stars,
            ),
            (
                StarSort::Name.label(),
                Action::SetStarSort(StarSort::Name),
                app.star_sort == StarSort::Name,
            ),
        ],
    );
    draw_list_filter(
        frame,
        app,
        theme,
        Rect::new(inner.x, inner.y + 4, inner.width, 1),
        hits,
    );
    draw_list_pagination(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, inner.y + 5, inner.width, 1),
    );
    let list = Rect::new(
        inner.x,
        inner.y + 7,
        inner.width,
        inner.height.saturating_sub(7),
    );
    app.layout.list = list;
    draw_cards(frame, app, theme, list, hits, EmptyKind::Starred);
}

fn draw_search_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    draw_page_title(
        frame,
        app,
        theme,
        inner,
        &icons::label(icons::SEARCH, "找仓库喵~"),
        "想找什么仓库喵~？告诉人家就好啦喵~",
        "刷新一下喵~",
        Action::Refresh,
        hits,
    );

    let search_rect = Rect::new(inner.x, inner.y + 3, inner.width, 3);
    draw_search_field(frame, app, theme, search_rect, hits);
    app.layout.search = search_rect;

    let sort_y = search_rect.y + 4;
    draw_sort_chips(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, sort_y, inner.width, 1),
        &icons::label(icons::CLOCK, "排序喵~"),
        &[
            (
                SearchSort::BestMatch.label(),
                Action::SetSearchSort(SearchSort::BestMatch),
                app.search_sort == SearchSort::BestMatch,
            ),
            (
                SearchSort::Stars.label(),
                Action::SetSearchSort(SearchSort::Stars),
                app.search_sort == SearchSort::Stars,
            ),
            (
                SearchSort::Updated.label(),
                Action::SetSearchSort(SearchSort::Updated),
                app.search_sort == SearchSort::Updated,
            ),
        ],
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &app.search_depth_label(),
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, sort_y + 1, inner.width, 1),
    );

    draw_list_filter(
        frame,
        app,
        theme,
        Rect::new(inner.x, sort_y + 2, inner.width, 1),
        hits,
    );

    draw_list_pagination(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, sort_y + 3, inner.width, 1),
    );
    let content = Rect::new(
        inner.x,
        sort_y + 4,
        inner.width,
        inner.height.saturating_sub(sort_y - inner.y + 4),
    );
    let detail_width = if area.width >= 116 {
        42
    } else if area.width >= 96 {
        35
    } else {
        0
    };
    let columns =
        Layout::horizontal([Constraint::Min(30), Constraint::Length(detail_width)]).split(content);
    app.layout.list = columns[0];
    draw_cards(frame, app, theme, columns[0], hits, EmptyKind::Search);
    if detail_width > 0 {
        draw_repository_detail(frame, app, theme, columns[1], hits);
        app.layout.detail = columns[1];
    }
}

fn draw_trending_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    let period = app.trending_period;
    let total = app
        .trending_total
        .map(format::human_count)
        .unwrap_or_else(|| "?".to_string());
    let language = app
        .language_filter
        .as_deref()
        .map(|language| format!(" · 只看 {language}"))
        .unwrap_or_default();
    draw_page_title(
        frame,
        app,
        theme,
        inner,
        &icons::label(icons::FLAME, "热门仓库喵~"),
        &format!(
            "GitHub 最近{}涨星最快的仓库，一共 {total} 个{language}喵~按 p 换时段哦喵~",
            period.short_label()
        ),
        "刷新一下喵~",
        Action::Refresh,
        hits,
    );

    let chips_y = inner.y + 3;
    draw_sort_chips(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, chips_y, inner.width, 1),
        &icons::label(icons::CLOCK, "时段喵~"),
        &[
            (
                TrendingPeriod::Daily.label(),
                Action::SetTrendingPeriod(TrendingPeriod::Daily),
                period == TrendingPeriod::Daily,
            ),
            (
                TrendingPeriod::Weekly.label(),
                Action::SetTrendingPeriod(TrendingPeriod::Weekly),
                period == TrendingPeriod::Weekly,
            ),
            (
                TrendingPeriod::Monthly.label(),
                Action::SetTrendingPeriod(TrendingPeriod::Monthly),
                period == TrendingPeriod::Monthly,
            ),
        ],
    );
    draw_list_filter(
        frame,
        app,
        theme,
        Rect::new(inner.x, chips_y + 1, inner.width, 1),
        hits,
    );
    draw_list_pagination(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, chips_y + 2, inner.width, 1),
    );
    let content = Rect::new(
        inner.x,
        chips_y + 3,
        inner.width,
        inner.height.saturating_sub(chips_y - inner.y + 3),
    );
    let detail_width = if area.width >= 116 {
        42
    } else if area.width >= 96 {
        35
    } else {
        0
    };
    let columns =
        Layout::horizontal([Constraint::Min(30), Constraint::Length(detail_width)]).split(content);
    app.layout.list = columns[0];
    draw_cards(frame, app, theme, columns[0], hits, EmptyKind::Trending);
    if detail_width > 0 {
        draw_repository_detail(frame, app, theme, columns[1], hits);
        app.layout.detail = columns[1];
    }
}

fn draw_account_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let width = area.width.saturating_sub(8).min(112);
    let left = area.x + (area.width.saturating_sub(width)) / 2;
    let inner = Rect::new(left, area.y + 1, width, area.height.saturating_sub(2));
    draw_page_title(
        frame,
        app,
        theme,
        inner,
        &icons::label(icons::USER, "账号喵~"),
        "这里放着你的 GitHub 小档案和 API 额度喵~",
        "刷新喵~",
        Action::Refresh,
        hits,
    );

    let (public, private, languages, stars) = app.repo_stats();
    let (starred, starred_languages, starred_stars) = app.starred_stats();
    let summary = format!(
        "{} 公开 {} · 私有 {} · 语言 {} 种 · 星标 {}    {} 收藏 {} · 语言 {} 种 · 星标 {}喵~",
        icons::REPO,
        public,
        private,
        languages,
        format::human_count(stars),
        icons::STAR,
        starred,
        starred_languages,
        format::human_count(starred_stars)
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&summary, inner.width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );

    let content = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(3),
    );
    if content.height < 8 {
        return;
    }
    let left_width = content.width.saturating_mul(11) / 20;
    let gap = 2;
    let profile_rect = Rect::new(content.x, content.y, left_width, content.height);
    let quota_rect = Rect::new(
        content.x + left_width + gap,
        content.y,
        content.width.saturating_sub(left_width + gap),
        content.height,
    );
    draw_account_profile(frame, app, theme, profile_rect, hits);
    draw_account_quota(frame, app, theme, quota_rect, hits);
}

fn draw_home_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let width = area.width.saturating_sub(6).min(128);
    let left = area.x + (area.width.saturating_sub(width)) / 2;
    let inner = Rect::new(left, area.y + 1, width, area.height.saturating_sub(2));
    if inner.width < 44 || inner.height < 10 {
        draw_center_message(
            frame,
            theme,
            area,
            "这里有点挤喵~",
            "把终端拉大一点，人家才摆得下首页喵~",
        );
        return;
    }

    let login = app
        .user
        .as_ref()
        .map(|user| user.login.clone())
        .or_else(|| app.login.clone())
        .unwrap_or_else(|| app.auth_label());
    let greeting = format!("欢迎回来，@{login}喵~");
    let title_width = inner.width.saturating_sub(22);
    frame.render_widget(
        Paragraph::new(format::truncate(&greeting, title_width as usize)).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, title_width, 1),
    );
    if inner.width > 82 {
        let palette = icons::label(icons::COMMAND, "Ctrl+K 命令喵~");
        let palette_width = format::text_width(&palette) as u16 + 4;
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(
                inner.x + inner.width.saturating_sub(palette_width),
                inner.y,
                palette_width,
                1,
            ),
            &palette,
            Action::OpenPalette,
            false,
        );
    }
    let summary = format!(
        "{} 个仓库 · {} 个收藏 · {} 条未读通知 · 置顶 {} 个喵~",
        app.repos.len(),
        app.starred.len(),
        app.unread_notifications(),
        app.pinned_list().len()
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&summary, inner.width as usize))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let content = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(3),
    );
    if content.height < 6 {
        return;
    }
    let stats_height = 4.min(content.height);
    draw_home_stats(
        frame,
        app,
        theme,
        hits,
        Rect::new(content.x, content.y, content.width, stats_height),
    );
    let below_y = content.y + stats_height + 1;
    let below_height = content.height.saturating_sub(stats_height + 1);
    if below_height < 2 {
        return;
    }
    let wide = content.width >= 104;
    let left_width = if wide {
        content.width.saturating_mul(58) / 100
    } else {
        content.width
    };
    draw_home_pins(
        frame,
        app,
        theme,
        hits,
        Rect::new(content.x, below_y, left_width, below_height),
    );
    if wide {
        let gap = 2;
        let right_col = Rect::new(
            content.x + left_width + gap,
            below_y,
            content.width.saturating_sub(left_width + gap),
            below_height,
        );
        draw_home_quick(frame, app, theme, hits, right_col);
    }
}

fn draw_home_stats(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    area: Rect,
) {
    let (public, private, _, _) = app.repo_stats();
    let (starred, _, stars) = app.starred_stats();
    let tiles = [
        (
            icons::REPO,
            "仓库喵~",
            app.repos.len().to_string(),
            format!("公开 {public} · 私有 {private}"),
            theme.color("blue"),
        ),
        (
            icons::STAR,
            "收藏喵~",
            starred.to_string(),
            format!("星标合计 {}", format::human_count(stars)),
            theme.color("mauve"),
        ),
        (
            icons::BELL,
            "未读喵~",
            app.unread_notifications().to_string(),
            format!("共 {} 条通知", app.notifications.len()),
            theme.color("peach"),
        ),
        (
            icons::DATABASE,
            "缓存喵~",
            format::format_size(app.cache_stats.bytes),
            format!("{} 个文件", app.cache_stats.files),
            theme.color("green"),
        ),
    ];
    let count = tiles.len() as u16;
    let gap = 1;
    let tile_width = area.width.saturating_sub(gap * count.saturating_sub(1)) / count.max(1);
    for (index, (icon, label, value, detail, accent)) in tiles.iter().enumerate() {
        let rect = Rect::new(
            area.x + index as u16 * (tile_width + gap),
            area.y,
            tile_width,
            area.height,
        );
        if rect.width < 8 {
            continue;
        }
        fill(frame, rect, theme.color("mantle"));
        frame.render_widget(
            Block::new()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.color("surface1"))),
            rect,
        );
        fill(
            frame,
            Rect::new(rect.x + 1, rect.y, 1, rect.height.saturating_sub(2)),
            *accent,
        );
        let text_width = rect.width.saturating_sub(4);
        frame.render_widget(
            Paragraph::new(format::truncate(
                &format!("{icon} {label}"),
                text_width as usize,
            ))
            .style(Style::default().fg(theme.color("overlay1"))),
            Rect::new(rect.x + 3, rect.y + 1, text_width, 1),
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    value.clone(),
                    Style::default().fg(*accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {detail}"),
                    Style::default().fg(theme.color("overlay0")),
                ),
            ])),
            Rect::new(rect.x + 3, rect.y + 2, text_width, 1),
        );
        register(hits, rect, Action::Refresh);
    }
}

fn draw_home_pins(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    area: Rect,
) {
    draw_section_header(
        frame,
        theme,
        Rect::new(area.x, area.y, area.width, 1),
        icons::PIN,
        "置顶仓库喵~",
        "按 P 钉住当前仓库",
    );
    let list = Rect::new(
        area.x,
        area.y + 2,
        area.width,
        area.height.saturating_sub(2),
    );
    if list.height == 0 {
        return;
    }
    let pins = app.pinned_list();
    if pins.is_empty() {
        draw_center_message(
            frame,
            theme,
            list,
            "还没有置顶仓库喵~",
            "去仓库列表挑一个，按 P 就能钉在首页啦喵~",
        );
        return;
    }
    app.home_selected = app.home_selected.min(pins.len().saturating_sub(1));
    let row_height = 2;
    let capacity = (list.height / row_height).max(1) as usize;
    let offset = app.home_selected.saturating_sub(capacity.saturating_sub(1));
    for (slot, (index, repo)) in pins
        .iter()
        .enumerate()
        .skip(offset)
        .take(capacity)
        .enumerate()
    {
        let rect = Rect::new(
            list.x,
            list.y + slot as u16 * row_height,
            list.width,
            row_height.min(list.height.saturating_sub(slot as u16 * row_height)),
        );
        if rect.height == 0 {
            break;
        }
        let action = Action::OpenPin(index);
        let selected = index == app.home_selected;
        let hovered = app.hover.as_ref() == Some(&action);
        if selected || hovered {
            fill(frame, rect, theme.color("surface0"));
        }
        if selected {
            fill(
                frame,
                Rect::new(rect.x, rect.y, 1, rect.height),
                theme.color("mauve"),
            );
        }
        let inner = Rect::new(rect.x + 2, rect.y, rect.width.saturating_sub(4), 1);
        let right = if repo.id == 0 {
            format!("{} 待取回", icons::CLOCK)
        } else {
            format!(
                "{} {}   {} {}",
                icons::STAR,
                format::human_count(repo.stargazers_count),
                icons::CODE,
                repo.language_label()
            )
        };
        let right_width = format::text_width(&right) as u16 + 1;
        let title = if repo.id == 0 {
            repo.display_full_name()
        } else {
            format!("{} / {}", repo.owner_name(), repo.name)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{} ", icons::PIN),
                    Style::default().fg(theme.color("mauve")),
                ),
                Span::styled(
                    format::truncate(&title, inner.width.saturating_sub(right_width) as usize),
                    Style::default()
                        .fg(theme.color("text"))
                        .add_modifier(Modifier::BOLD),
                ),
            ])),
            Rect::new(inner.x, inner.y, inner.width.saturating_sub(right_width), 1),
        );
        frame.render_widget(
            Paragraph::new(right)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
        if rect.height > 1 {
            let detail = if repo.description_text().is_empty() {
                "这个仓库还没有简介喵~".to_string()
            } else {
                format::clean_text(repo.description_text())
            };
            frame.render_widget(
                Paragraph::new(format::truncate(&detail, inner.width as usize))
                    .style(Style::default().fg(theme.color("subtext0"))),
                Rect::new(inner.x, inner.y + 1, inner.width, 1),
            );
        }
        register(hits, rect, action);
    }
}

fn draw_home_quick(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    area: Rect,
) {
    draw_section_header(
        frame,
        theme,
        Rect::new(area.x, area.y, area.width, 1),
        icons::SPARKLE,
        "快捷入口喵~",
        "点一下就到",
    );
    let grid_height = 7.min(area.height.saturating_sub(2));
    let grid = Rect::new(area.x, area.y + 2, area.width, grid_height);
    let items = [
        (
            icons::label(icons::REPO, "仓库喵~"),
            Action::Navigate(Page::Repositories),
        ),
        (
            icons::label(icons::SEARCH, "搜索喵~"),
            Action::Navigate(Page::Search),
        ),
        (
            icons::label(icons::FLAME, "热门喵~"),
            Action::Navigate(Page::Trending),
        ),
        (
            icons::label(icons::BELL, "通知喵~"),
            Action::Navigate(Page::Notifications),
        ),
        (
            icons::label(icons::CHART, "洞察喵~"),
            Action::Navigate(Page::Insights),
        ),
        (
            icons::label(icons::SETTINGS, "设置喵~"),
            Action::Navigate(Page::Settings),
        ),
        (icons::label(icons::THEME, "主题喵~"), Action::OpenThemes),
        (icons::label(icons::RANDOM, "随机喵~"), Action::RandomRepo),
    ];
    let row_gap = 1;
    let column_width = grid.width.saturating_sub(1) / 2;
    for (index, (label, action)) in items.iter().enumerate() {
        let column = index % 2;
        let row = index / 2;
        let rect = Rect::new(
            grid.x + column as u16 * (column_width + 1),
            grid.y + row as u16 * (1 + row_gap),
            column_width,
            1,
        );
        if rect.y >= grid.y + grid.height || rect.width < 8 {
            break;
        }
        draw_button(frame, app, theme, hits, rect, label, action.clone(), false);
    }

    let languages_y = area.y + 2 + grid_height + 1;
    if languages_y + 2 >= area.y + area.height {
        return;
    }
    let languages = app.language_overview();
    draw_section_header(
        frame,
        theme,
        Rect::new(area.x, languages_y, area.width, 1),
        icons::CODE,
        "语言分布喵~",
        "仓库 + 收藏",
    );
    let bars = Rect::new(
        area.x,
        languages_y + 2,
        area.width,
        area.height.saturating_sub(languages_y - area.y + 2),
    );
    draw_language_bars(frame, theme, bars, &languages);
}

fn draw_insights_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let width = area.width.saturating_sub(8).min(118);
    let left = area.x + (area.width.saturating_sub(width)) / 2;
    let inner = Rect::new(left, area.y + 1, width, area.height.saturating_sub(2));
    if inner.width < 44 || inner.height < 10 {
        draw_center_message(
            frame,
            theme,
            area,
            "这里有点挤喵~",
            "把终端拉大一点，人家才摆得下洞察页喵~",
        );
        return;
    }
    draw_page_title(
        frame,
        app,
        theme,
        inner,
        &icons::label(icons::CHART, "数据洞察喵~"),
        "从仓库、收藏、通知和缓存里算出来的小报告喵~",
        "刷新喵~",
        Action::Refresh,
        hits,
    );
    let (public, private, languages, stars) = app.repo_stats();
    let (starred, starred_languages, starred_stars) = app.starred_stats();
    let summary = format!(
        "{} 仓库（公开 {public} · 私有 {private}）· {} 语言 · {} 星标    {} 收藏（{} 语言 · {} 星标）· {} 未读通知喵~",
        app.repos.len(),
        languages,
        format::human_count(stars),
        starred,
        starred_languages,
        format::human_count(starred_stars),
        app.unread_notifications()
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&summary, inner.width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );

    let content = Rect::new(
        inner.x,
        inner.y + 4,
        inner.width,
        inner.height.saturating_sub(4),
    );
    if content.height < 4 {
        return;
    }
    if content.width >= 94 {
        let left_width = content.width.saturating_mul(56) / 100;
        let gap = 2;
        let left_col = Rect::new(content.x, content.y, left_width, content.height);
        let right_col = Rect::new(
            content.x + left_width + gap,
            content.y,
            content.width.saturating_sub(left_width + gap),
            content.height,
        );
        draw_insights_left(frame, app, theme, left_col);
        draw_insights_right(frame, app, theme, right_col);
    } else {
        draw_insights_left(frame, app, theme, content);
    }
}

fn draw_insights_left(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    draw_section_header(
        frame,
        theme,
        Rect::new(area.x, area.y, area.width, 1),
        icons::CODE,
        "语言分布喵~",
        "仓库 + 收藏",
    );
    let languages = app.language_overview();
    let language_height = area.height.saturating_sub(3).min(8);
    draw_language_bars(
        frame,
        theme,
        Rect::new(area.x, area.y + 2, area.width, language_height),
        &languages,
    );

    let top_y = area.y + 2 + language_height + 1;
    if top_y >= area.y + area.height {
        return;
    }
    draw_section_header(
        frame,
        theme,
        Rect::new(area.x, top_y, area.width, 1),
        icons::STAR,
        "星标排行喵~",
        "按 star 数",
    );
    let top = app.top_repos_by_stars(5);
    if top.is_empty() {
        frame.render_widget(
            Paragraph::new("还没有仓库数据喵~").style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x, top_y + 2, area.width, 1),
        );
        return;
    }
    for (index, repo) in top.iter().enumerate() {
        let y = top_y + 2 + index as u16;
        if y >= area.y + area.height {
            break;
        }
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{:>2}. ", index + 1),
                    Style::default().fg(theme.color("overlay0")),
                ),
                Span::styled(
                    format::truncate(
                        &repo.display_full_name(),
                        area.width.saturating_sub(18) as usize,
                    ),
                    Style::default().fg(theme.color("text")),
                ),
                Span::styled(
                    format!(
                        "  {} {}",
                        icons::STAR,
                        format::human_count(repo.stargazers_count)
                    ),
                    Style::default().fg(theme.color("yellow")),
                ),
            ])),
            Rect::new(area.x, y, area.width, 1),
        );
    }
}

fn draw_insights_right(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    draw_section_header(
        frame,
        theme,
        Rect::new(area.x, area.y, area.width, 1),
        icons::DATABASE,
        "缓存与额度喵~",
        "本地小账本",
    );
    let mut y = area.y + 2;
    let (public, private, languages, _) = app.repo_stats();
    let (starred, _, _) = app.starred_stats();
    let rows = [
        (
            icons::REPO,
            "仓库".to_string(),
            format!("{} 个 · 公开 {public} · 私有 {private}", app.repos.len()),
            theme.color("blue"),
        ),
        (
            icons::STAR,
            "收藏".to_string(),
            format!("{starred} 个 · {languages} 种语言"),
            theme.color("mauve"),
        ),
        (
            icons::BELL,
            "通知".to_string(),
            format!(
                "{} 条未读 / {} 条",
                app.unread_notifications(),
                app.notifications.len()
            ),
            theme.color("peach"),
        ),
        (
            icons::DATABASE,
            "缓存".to_string(),
            format!(
                "{} 个文件 · {}",
                app.cache_stats.files,
                format::format_size(app.cache_stats.bytes)
            ),
            theme.color("green"),
        ),
    ];
    for (icon, label, value, color) in rows {
        if y >= area.y + area.height {
            return;
        }
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{icon} "), Style::default().fg(color)),
                Span::styled(label, Style::default().fg(theme.color("subtext0"))),
            ])),
            Rect::new(area.x, y, 12, 1),
        );
        frame.render_widget(
            Paragraph::new(format::truncate(
                &value,
                area.width.saturating_sub(13) as usize,
            ))
            .style(Style::default().fg(theme.color("text"))),
            Rect::new(area.x + 12, y, area.width.saturating_sub(12), 1),
        );
        y += 1;
    }

    y += 1;
    if y + 3 >= area.y + area.height {
        return;
    }
    draw_section_header(
        frame,
        theme,
        Rect::new(area.x, y, area.width, 1),
        icons::GLOBE,
        "API 额度喵~",
        "GitHub 限量",
    );
    y += 2;
    let Some(rate) = app.rate_limit.as_ref() else {
        frame.render_widget(
            Paragraph::new("还没取到额度信息喵~按 r 刷新看看")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x, y, area.width, 1),
        );
        return;
    };
    for (label, info) in rate.items() {
        if y >= area.y + area.height {
            break;
        }
        let ratio = if info.limit == 0 {
            0.0
        } else {
            info.remaining as f32 / info.limit as f32
        };
        let color = if ratio > 0.5 {
            theme.color("green")
        } else if ratio > 0.2 {
            theme.color("yellow")
        } else {
            theme.color("red")
        };
        draw_metric_bar(
            frame,
            theme,
            Rect::new(area.x, y, area.width, 1),
            label,
            ratio,
            &format!("{}/{}", info.remaining, info.limit),
            color,
            icons::NETWORK,
        );
        y += 2;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_metric_bar(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    label: &str,
    ratio: f32,
    value: &str,
    color: Color,
    icon: &str,
) {
    if area.width < 12 || area.height == 0 {
        return;
    }
    let label_width = 12_u16.min(area.width / 3).max(6);
    let value_width = (format::text_width(value) as u16)
        .min(area.width / 2)
        .max(3);
    let bar_width = area.width.saturating_sub(label_width + value_width + 1);
    let inner_bar = bar_width.saturating_sub(2).max(1) as usize;
    let filled = (inner_bar as f32 * ratio.clamp(0.0, 1.0)).round() as usize;
    frame.render_widget(
        Paragraph::new(format::truncate(label, label_width as usize))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(area.x, area.y, label_width, 1),
    );
    let mut spans = vec![Span::styled(format!("{icon} "), Style::default().fg(color))];
    spans.extend(gradient_chars(filled, "▰", color, color));
    spans.extend(gradient_chars(
        inner_bar.saturating_sub(filled),
        "▱",
        theme.color("surface1"),
        theme.color("surface1"),
    ));
    frame.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect::new(area.x + label_width, area.y, bar_width, 1),
    );
    frame.render_widget(
        Paragraph::new(value.to_string())
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            area.x + area.width.saturating_sub(value_width),
            area.y,
            value_width,
            1,
        ),
    );
}

fn draw_language_bars(frame: &mut Frame, theme: &Theme, area: Rect, languages: &[(String, usize)]) {
    if languages.is_empty() || area.height == 0 {
        frame.render_widget(
            Paragraph::new("还没有语言数据喵~").style(Style::default().fg(theme.color("overlay0"))),
            area,
        );
        return;
    }
    let max = languages
        .iter()
        .map(|(_, count)| *count)
        .max()
        .unwrap_or(1)
        .max(1);
    let palette = [
        theme.color("blue"),
        theme.color("mauve"),
        theme.color("green"),
        theme.color("peach"),
        theme.color("sky"),
        theme.color("pink"),
        theme.color("yellow"),
        theme.color("teal"),
    ];
    for (index, (language, count)) in languages.iter().take(area.height as usize).enumerate() {
        let y = area.y + index as u16;
        let color = palette[index % palette.len()];
        draw_metric_bar(
            frame,
            theme,
            Rect::new(area.x, y, area.width, 1),
            language,
            *count as f32 / max as f32,
            &count.to_string(),
            color,
            icons::CODE,
        );
    }
}

fn draw_section_header(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    icon: &str,
    title: &str,
    note: &str,
) {
    if area.width == 0 {
        return;
    }
    let mut spans = vec![
        Span::styled(format!("{icon} "), Style::default().fg(theme.color("blue"))),
        Span::styled(
            title.to_string(),
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ", Style::default()),
    ];
    let used = format::text_width(&format!("{icon} {title}  "));
    let note = if area.width as usize > used + format::text_width(note) + 6 {
        note.to_string()
    } else {
        String::new()
    };
    let note_width = format::text_width(&note);
    let rule_width = (area.width as usize)
        .saturating_sub(used + note_width + 2)
        .max(1);
    spans.extend(gradient_chars(
        rule_width,
        "─",
        theme.color("surface1"),
        theme.color("surface2"),
    ));
    if !note.is_empty() {
        spans.push(Span::styled(
            format!("  {note}"),
            Style::default().fg(theme.color("overlay0")),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_profile_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let width = area.width.saturating_sub(8).min(118);
    let left = area.x + (area.width.saturating_sub(width)) / 2;
    let inner = Rect::new(left, area.y + 1, width, area.height.saturating_sub(2));
    if inner.height < 10 || inner.width < 40 {
        draw_center_message(
            frame,
            theme,
            area,
            "这里有点挤喵~",
            "把终端拉大一点，人家才摆得下主页喵~",
        );
        return;
    }

    let back = icons::label(icons::BACK, "返回喵~");
    let back_width = format::text_width(&back) as u16 + 4;
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, inner.y, back_width, 1),
        &back,
        Action::Back,
        false,
    );
    let login = app.profile_login.clone().unwrap_or_default();
    let title = if login.is_empty() {
        "用户主页喵~".to_string()
    } else {
        format!("@{} 的主页喵~", format::clean_text(&login))
    };
    let title_x = inner.x + back_width + 2;
    let refresh = icons::label(icons::REFRESH, "刷新喵~");
    let refresh_width = format::text_width(&refresh) as u16 + 4;
    let title_width = inner.width.saturating_sub(back_width + refresh_width + 4);
    frame.render_widget(
        Paragraph::new(format::truncate(&title, title_width as usize)).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(title_x, inner.y, title_width, 1),
    );
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(
            inner.x + inner.width.saturating_sub(refresh_width),
            inner.y,
            refresh_width,
            1,
        ),
        &refresh,
        Action::Refresh,
        false,
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            "这个人的小档案和全部公开仓库喵~",
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let content = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(3),
    );
    if content.height < 6 {
        return;
    }
    if content.width >= 96 {
        let profile_width = content.width.saturating_mul(2) / 5;
        let gap = 2;
        let profile_rect = Rect::new(content.x, content.y, profile_width, content.height);
        let list_rect = Rect::new(
            content.x + profile_width + gap,
            content.y,
            content.width.saturating_sub(profile_width + gap),
            content.height,
        );
        draw_profile_card(frame, app, theme, profile_rect, hits);
        draw_profile_repos(frame, app, theme, list_rect, hits);
    } else {
        let profile_height = (content.height.saturating_mul(2) / 5)
            .clamp(6, 12)
            .min(content.height.saturating_sub(4));
        let profile_rect = Rect::new(content.x, content.y, content.width, profile_height);
        let list_rect = Rect::new(
            content.x,
            content.y + profile_height + 1,
            content.width,
            content.height.saturating_sub(profile_height + 1),
        );
        draw_profile_card(frame, app, theme, profile_rect, hits);
        draw_profile_repos(frame, app, theme, list_rect, hits);
    }
}

fn draw_profile_card(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 6 || inner.width < 12 {
        if let Some(user) = app.profile_user.as_ref() {
            frame.render_widget(
                Paragraph::new(format::truncate(
                    &format!("@{} · {}", user.login, format::human_count(user.followers)),
                    inner.width as usize,
                ))
                .style(Style::default().fg(theme.color("subtext0"))),
                Rect::new(inner.x, inner.y, inner.width, 1),
            );
        }
        return;
    }
    frame.render_widget(
        Paragraph::new(icons::label(icons::USER, "用户小档案喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let Some(user) = app.profile_user.as_ref() else {
        let detail = if app.profile_user_loading {
            format!("{} 人家正在问 GitHub 喵~...", app.spinner())
        } else {
            app.profile_error
                .as_deref()
                .unwrap_or("按 r 刷新一下，人家再去取喵~")
                .to_string()
        };
        draw_center_message(frame, theme, area, "资料还没拿来喵~", &detail);
        return;
    };
    frame.render_widget(
        Paragraph::new(format::truncate(user.display_name(), inner.width as usize)).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("@{}", user.login),
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("blue"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );

    let mut y = inner.y + 4;
    let compact = inner.height < 12;
    if !compact {
        let bio = user
            .bio
            .as_deref()
            .map(format::clean_text)
            .filter(|bio| !bio.trim().is_empty())
            .unwrap_or_else(|| "还没有写简介喵~".to_string());
        for line in format::wrap_text(&bio, inner.width as usize, 3) {
            if y >= inner.y + inner.height - 5 {
                break;
            }
            frame.render_widget(
                Paragraph::new(line).style(Style::default().fg(theme.color("subtext0"))),
                Rect::new(inner.x, y, inner.width, 1),
            );
            y += 1;
        }
        y += 1;
        let facts = [
            (icons::COGS, "公司喵~", user.company.clone()),
            (icons::GLOBE, "位置喵~", user.location.clone()),
            (icons::NETWORK, "博客喵~", user.blog.clone()),
            (icons::CLOCK, "加入喵~", Some(user.joined_label())),
        ];
        for (icon, label, value) in facts {
            if y >= inner.y + inner.height - 5 {
                break;
            }
            let Some(value) = value
                .map(|value| format::clean_text(&value))
                .filter(|value| !value.trim().is_empty())
            else {
                continue;
            };
            frame.render_widget(
                Paragraph::new(format::truncate(
                    &format!("{icon} {label}  {value}"),
                    inner.width as usize,
                ))
                .style(Style::default().fg(theme.color("subtext0"))),
                Rect::new(inner.x, y, inner.width, 1),
            );
            y += 1;
        }
    }

    let stars = app
        .profile_repos
        .iter()
        .map(|repo| repo.stargazers_count)
        .sum::<u64>();
    let stats_y = (inner.y + inner.height).saturating_sub(3);
    let stats = format!(
        "{} 仓库 {}   {} 关注 {}   粉丝 {}   星标 {}喵~",
        icons::REPO,
        user.public_repos,
        icons::USER,
        user.following,
        user.followers,
        format::human_count(stars)
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&stats, inner.width as usize))
            .style(Style::default().fg(theme.color("text"))),
        Rect::new(inner.x, stats_y, inner.width, 1),
    );
    let open = icons::label(icons::EXTERNAL, "打开 GitHub 主页喵~");
    let open_width = format::text_width(&open) as u16 + 4;
    if stats_y + 1 < inner.y + inner.height && open_width < inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(inner.x, stats_y + 1, open_width, 1),
            &open,
            Action::OpenExternal(user.html_url.clone()),
            true,
        );
    }
}

fn draw_profile_repos(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.height < 4 || area.width < 20 {
        return;
    }
    let count = app.profile_repos.len();
    let loading = app.profile_repos_loading;
    let title = if loading && count == 0 {
        format!("{} 人家正在搬仓库喵~...", app.spinner())
    } else if loading {
        format!(
            "{} 全部仓库 {} 个 · {} 刷新中喵~",
            icons::REPO,
            count,
            app.spinner()
        )
    } else {
        format!("{} 全部仓库 {} 个喵~", icons::REPO, count)
    };
    frame.render_widget(
        Paragraph::new(format::truncate(&title, area.width as usize)).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x, area.y, area.width, 1),
    );
    draw_list_pagination(
        frame,
        app,
        theme,
        hits,
        Rect::new(area.x, area.y + 1, area.width, 1),
    );
    let list = Rect::new(
        area.x,
        area.y + 3,
        area.width,
        area.height.saturating_sub(3),
    );
    app.layout.list = list;
    draw_cards(frame, app, theme, list, hits, EmptyKind::Profile);
}

fn draw_account_profile(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 6 || inner.width < 12 {
        return;
    }
    let Some(user) = app.user.as_ref() else {
        draw_center_message(
            frame,
            theme,
            area,
            "还没有登录喵~",
            "先在终端跑一下 gh auth login，再来看看嘛喵~",
        );
        return;
    };

    frame.render_widget(
        Paragraph::new(icons::label(icons::USER, "人家的小档案喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(user.display_name(), inner.width as usize)).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!("@{}", user.login)).style(Style::default().fg(theme.color("blue"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );

    let mut y = inner.y + 4;
    let bio = user
        .bio
        .as_deref()
        .map(format::clean_text)
        .filter(|bio| !bio.trim().is_empty())
        .unwrap_or_else(|| "还没有写简介喵~".to_string());
    for line in format::wrap_text(&bio, inner.width as usize, 3) {
        frame.render_widget(
            Paragraph::new(line).style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }

    let facts = [
        (icons::COGS, "公司喵~", user.company.clone()),
        (icons::GLOBE, "位置喵~", user.location.clone()),
        (icons::NETWORK, "博客喵~", user.blog.clone()),
        (icons::INFO, "邮箱喵~", user.email.clone()),
        (
            icons::COMMENT,
            "推特喵~",
            user.twitter_username
                .as_deref()
                .map(|name| format!("@{name}")),
        ),
        (
            icons::CODE,
            "公开 Gist喵~",
            Some(user.public_gists.to_string()),
        ),
        (icons::CLOCK, "加入喵~", Some(user.joined_label())),
        (
            icons::STAR,
            "计划喵~",
            user.plan
                .as_ref()
                .map(|plan| format!("{} · {} 个私有仓库喵~", plan.name, plan.private_repos)),
        ),
    ];
    y += 1;
    for (icon, label, value) in facts {
        if y >= inner.y + inner.height - 7 {
            break;
        }
        let value = value
            .map(|value| format::clean_text(&value))
            .filter(|value| !value.trim().is_empty());
        let Some(value) = value else {
            continue;
        };
        frame.render_widget(
            Paragraph::new(format::truncate(
                &format!("{icon} {label}  {value}"),
                inner.width as usize,
            ))
            .style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }

    let stats_y = inner.y + inner.height.saturating_sub(6);
    let (public, private, languages, stars) = app.repo_stats();
    let (starred, starred_languages, starred_stars) = app.starred_stats();
    let stats = format!(
        "{} 公开 {}   私有 {}   语言 {} 种   {} 星标 {}喵~",
        icons::REPO,
        public,
        private,
        languages,
        icons::STAR,
        format::human_count(stars)
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&stats, inner.width as usize))
            .style(Style::default().fg(theme.color("text"))),
        Rect::new(inner.x, stats_y, inner.width, 1),
    );
    let follow = format!(
        "{} 正在关注 {}   {} 粉丝 {}   {} 收藏 {}喵~",
        icons::USER,
        user.following,
        icons::EYE,
        user.followers,
        icons::STAR,
        starred
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&follow, inner.width as usize))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, stats_y + 1, inner.width, 1),
    );
    let extra = format!(
        "{} 收藏星标 {}   {} 收藏语言 {} 种   公开 Gist {}喵~",
        icons::STAR,
        format::human_count(starred_stars),
        icons::CODE,
        starred_languages,
        user.public_gists
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&extra, inner.width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, stats_y + 2, inner.width, 1),
    );

    let open = icons::label(icons::EXTERNAL, "打开主页喵~");
    let open_width = format::text_width(&open) as u16 + 4;
    if stats_y + 4 < inner.y + inner.height {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(inner.x, stats_y + 4, open_width, 1),
            &open,
            Action::OpenExternal(user.html_url.clone()),
            true,
        );
    }
}

fn draw_account_quota(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 6 || inner.width < 12 {
        return;
    }
    frame.render_widget(
        Paragraph::new(icons::label(icons::NETWORK, "GitHub API 额度喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let source = if app.client.token_present() {
        "凭据来自 gh auth token，额度按当前账号算喵~"
    } else {
        "还没登录，看到的是匿名额度喵~"
    };
    frame.render_widget(
        Paragraph::new(format::truncate(source, inner.width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let Some(rate) = app.rate_limit.as_ref() else {
        let detail = if app.rate_loading {
            format!("{} 人家正在问 GitHub 喵~...", app.spinner())
        } else {
            "点一下刷新，人家才去问 GitHub 喵~".to_string()
        };
        draw_center_message(frame, theme, area, "额度还没拿来喵~", &detail);
        return;
    };

    let now = crate::cache::now_epoch();
    let mut y = inner.y + 3;
    for (label, info) in rate.items() {
        if y + 2 >= inner.y + inner.height {
            break;
        }
        let percent = if info.limit == 0 {
            0
        } else {
            (info.remaining as f64 / info.limit as f64 * 100.0).round() as u32
        };
        let color = if percent >= 50 {
            theme.color("green")
        } else if percent >= 20 {
            theme.color("yellow")
        } else {
            theme.color("red")
        };
        draw_metric_bar(
            frame,
            theme,
            Rect::new(inner.x, y, inner.width, 1),
            label,
            percent as f32 / 100.0,
            &format!("{} / {} · {}%", info.remaining, info.limit, percent),
            color,
            icons::NETWORK,
        );
        frame.render_widget(
            Paragraph::new(format!(
                "已用 {} · {}喵~",
                info.used,
                format::reset_label(info.reset, now)
            ))
            .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, y + 1, inner.width, 1),
        );
        y += 3;
    }

    let note = format!("{} 额度接口不消耗额度，刷新随便点喵~", icons::INFO);
    if y < inner.y + inner.height {
        frame.render_widget(
            Paragraph::new(format::truncate(&note, inner.width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
        );
    }
    let refresh = icons::label(icons::REFRESH, "刷新额度喵~");
    let refresh_width = format::text_width(&refresh) as u16 + 4;
    if inner.y + inner.height >= 2 {
        let rect = Rect::new(
            inner.x + inner.width.saturating_sub(refresh_width),
            inner.y,
            refresh_width,
            1,
        );
        draw_button(
            frame,
            app,
            theme,
            hits,
            rect,
            &refresh,
            Action::RefreshRateLimit,
            false,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_filter_line(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
    input: &TextInput,
    focused: bool,
    hovered: bool,
    action: Action,
    placeholder: &str,
) {
    if area.width < 10 || area.height == 0 {
        return;
    }
    let background = if focused || hovered {
        theme.color("surface1")
    } else {
        theme.color("surface0")
    };
    fill(frame, area, background);
    if focused {
        fill(frame, Rect::new(area.x, area.y, 1, 1), theme.color("blue"));
    }
    let icon = format!("{} ", icons::SEARCH);
    let icon_width = format::text_width(&icon) as u16;
    frame.render_widget(
        Paragraph::new(icon).style(Style::default().fg(if focused || hovered {
            theme.color("blue")
        } else {
            theme.color("subtext0")
        })),
        Rect::new(area.x + 2, area.y, icon_width, 1),
    );
    let input_width = area.width.saturating_sub(icon_width + 4);
    let (visible, cursor_column) = if input.is_empty() {
        (format::truncate(placeholder, input_width as usize), 0)
    } else {
        visible_input(input, focused, input_width)
    };
    let style = if input.is_empty() {
        Style::default().fg(theme.color("overlay0"))
    } else {
        Style::default().fg(theme.color("text"))
    };
    frame.render_widget(
        Paragraph::new(visible).style(style),
        Rect::new(area.x + 2 + icon_width, area.y, input_width, 1),
    );
    register(hits, area, action);
    if focused {
        let x = area
            .x
            .saturating_add(2 + icon_width + cursor_column.min(input_width.saturating_sub(1)));
        frame.set_cursor_position(Position::new(x, area.y));
    }
}

fn draw_list_filter(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    draw_filter_line(
        frame,
        theme,
        area,
        hits,
        app.current_filter(),
        app.list_filter_focus,
        app.hover.as_ref() == Some(&Action::ListFilterFocus),
        Action::ListFilterFocus,
        "按 / 输入关键词筛选，Enter 离开喵~",
    );
}

fn draw_search_field(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let border_color = if app.search_focus {
        theme.color("blue")
    } else {
        theme.color("surface1")
    };
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
        area,
    );
    let button_width = 9;
    let input_width = area.width.saturating_sub(button_width + 5);
    let input_rect = Rect::new(area.x + 2, area.y + 1, input_width, 1);
    let placeholder = "想找什么喵~？输入后按回车嘛";
    let (visible, cursor_column) = if app.search_input.is_empty() {
        (format::truncate(placeholder, input_width as usize), 0_u16)
    } else {
        visible_input(&app.search_input, app.search_focus, input_width)
    };
    let text_style = if app.search_input.is_empty() {
        Style::default().fg(theme.color("overlay0"))
    } else {
        Style::default().fg(theme.color("text"))
    };
    frame.render_widget(Paragraph::new(visible).style(text_style), input_rect);
    let field_hit = Rect::new(area.x, area.y, area.width, area.height);
    register(hits, field_hit, Action::SearchFocus);
    let button_rect = Rect::new(
        area.x + area.width.saturating_sub(button_width + 1),
        area.y + 1,
        button_width,
        1,
    );
    draw_button(
        frame,
        app,
        theme,
        hits,
        button_rect,
        &icons::label(icons::SEARCH, "找找喵~"),
        Action::SubmitSearch,
        true,
    );
    if app.search_focus {
        let x = input_rect
            .x
            .saturating_add(cursor_column.min(input_width.saturating_sub(1)));
        frame.set_cursor_position(Position::new(x, input_rect.y));
    }
}

fn draw_list_pagination(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    area: Rect,
) {
    if area.height == 0 || area.width < 26 {
        return;
    }
    let prev = icons::label(icons::BACK, "上一页喵~");
    let next = icons::label(icons::EXTERNAL, "下一页喵~");
    let prev_width = format::text_width(&prev) as u16 + 4;
    let next_width = format::text_width(&next) as u16 + 4;
    let buttons_width = prev_width + next_width + 2;
    let label_width = area.width.saturating_sub(buttons_width + 1);
    frame.render_widget(
        Paragraph::new(format::truncate(&app.page_label(), label_width as usize))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(area.x, area.y, label_width, 1),
    );
    let prev_rect = Rect::new(area.x + area.width - buttons_width, area.y, prev_width, 1);
    draw_button(
        frame,
        app,
        theme,
        hits,
        prev_rect,
        &prev,
        Action::PrevListPage,
        false,
    );
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(prev_rect.x + prev_width + 2, area.y, next_width, 1),
        &next,
        Action::NextListPage,
        true,
    );
}

fn draw_cards(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
    empty: EmptyKind,
) {
    if area.height < CARD_HEIGHT {
        return;
    }
    let indices = app.visible_indices();
    let viewport = (area.height / CARD_STEP).max(1) as usize;
    app.viewport_cards = viewport;
    let selected = app.selected_index();

    if indices.is_empty() {
        app.list_offset = 0;
        draw_empty_state(frame, app, theme, area, hits, empty);
        return;
    }

    let page = (selected / viewport).min((indices.len() - 1) / viewport);
    app.list_offset = page * viewport;

    let hovered = app.hover.clone();
    for (row, (slot, index)) in indices
        .iter()
        .enumerate()
        .skip(app.list_offset)
        .take(viewport)
        .enumerate()
    {
        let card = Rect::new(
            area.x,
            area.y + row as u16 * CARD_STEP,
            area.width.saturating_sub(1),
            CARD_HEIGHT,
        );
        let action = Action::SelectVisible(slot);
        let is_selected = slot == selected;
        let is_hovered = hovered.as_ref() == Some(&action);
        let repo = app.raw_repo(*index);
        let owner_action = Action::OpenProfile(repo.owner_name().to_string());
        let owner_hovered = hovered.as_ref() == Some(&owner_action);
        register(hits, card, action);
        draw_repo_card(
            frame,
            theme,
            card,
            &repo,
            is_selected,
            is_hovered,
            owner_hovered,
            app.is_pinned(&repo),
            hits,
        );
    }

    if indices.len() > viewport {
        let scroll_rect = Rect::new(
            area.x + area.width.saturating_sub(1),
            area.y,
            1,
            area.height,
        );
        let track = Style::default().fg(theme.color("surface0"));
        for y in 0..scroll_rect.height {
            frame.render_widget(
                Paragraph::new("|").style(track),
                Rect::new(scroll_rect.x, scroll_rect.y + y, 1, 1),
            );
        }
        let ratio = viewport as f32 / indices.len() as f32;
        let thumb_height = (area.height as f32 * ratio).max(1.0) as u16;
        let max_offset = indices.len().saturating_sub(viewport).max(1);
        let offset = (app.list_offset as f32 / max_offset as f32
            * (area.height.saturating_sub(thumb_height)) as f32) as u16;
        for y in 0..thumb_height {
            frame.render_widget(
                Paragraph::new("|").style(Style::default().fg(theme.color("overlay0"))),
                Rect::new(scroll_rect.x, scroll_rect.y + offset + y, 1, 1),
            );
        }
    }
}

#[derive(Clone, Copy)]
enum EmptyKind {
    Repositories,
    Starred,
    Search,
    Trending,
    Profile,
}

fn draw_empty_state(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
    kind: EmptyKind,
) {
    if matches!(kind, EmptyKind::Search) && app.search_submitted.is_none() {
        draw_search_history(frame, app, theme, area, hits);
        return;
    }
    let loading = match kind {
        EmptyKind::Repositories => app.repos_loading,
        EmptyKind::Starred => app.starred_loading,
        EmptyKind::Search => app.search_loading,
        EmptyKind::Trending => app.trending_loading,
        EmptyKind::Profile => app.profile_repos_loading,
    };
    let (title, detail) = if loading {
        ("人家正在搬数据喵~...", "第一次同步会慢一点点，等一等哦喵~")
    } else if !app.client.token_present()
        && !matches!(kind, EmptyKind::Trending | EmptyKind::Profile)
    {
        (
            "还没有登录 GitHub喵~",
            "在终端跑一下 gh auth login，再重新打开人家嘛喵~",
        )
    } else if matches!(kind, EmptyKind::Profile) && app.profile_error.is_some() {
        (
            "这个人的资料没取到喵~",
            app.profile_error.as_deref().unwrap_or("按 r 再试一次嘛喵~"),
        )
    } else if app.list_filter_active() {
        (
            "没有匹配的仓库喵~",
            "换个关键词，或者按 / 回到筛选框改一改喵~",
        )
    } else {
        match kind {
            EmptyKind::Repositories => (
                "这里还没有仓库呢喵~",
                "先去 GitHub 建一个，或者按 r 刷新看看嘛喵~",
            ),
            EmptyKind::Starred => (
                "还没有收藏任何仓库喵~",
                "去 GitHub 点一颗星星，这里就会热闹起来啦喵~",
            ),
            EmptyKind::Search => ("没有搜到仓库喵~", "换个关键词再试试嘛喵~"),
            EmptyKind::Trending => ("热门仓库还没到喵~", "按 r 刷新，或者稍等一下再看看嘛喵~"),
            EmptyKind::Profile => (
                "这个人还没有公开仓库喵~",
                "按 r 刷新，或者去 GitHub 上看看嘛喵~",
            ),
        }
    };
    draw_center_message(frame, theme, area, &format!("ᓚᘏᗢ  {title}"), detail);
}

fn draw_search_history(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if app.search_submitted.is_none() {
        let title = if app.search_loading {
            "人家正在找仓库喵~..."
        } else {
            "还没有开始搜索呢喵~"
        };
        let mut y = area.y + 2;
        frame.render_widget(
            Paragraph::new(title).style(
                Style::default()
                    .fg(theme.color("subtext1"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(area.x + 1, y, area.width.saturating_sub(2), 1),
        );
        y += 1;
        frame.render_widget(
            Paragraph::new("在上面输入关键词，按回车人家就去找啦喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 1, y, area.width.saturating_sub(2), 1),
        );
        y += 3;
        if !app.config.search_history.is_empty() {
            frame.render_widget(
                Paragraph::new("人家最近找过喵~").style(
                    Style::default()
                        .fg(theme.color("subtext0"))
                        .add_modifier(Modifier::BOLD),
                ),
                Rect::new(area.x + 1, y, area.width.saturating_sub(2), 1),
            );
            y += 1;
            let history = app.config.search_history.clone();
            for query in history {
                if y >= area.y + area.height {
                    break;
                }
                let rect = Rect::new(area.x, y, area.width.saturating_sub(1), 1);
                let action = Action::SearchHistory(query.clone());
                let hovered = app.hover.as_ref() == Some(&action);
                if hovered {
                    fill(frame, rect, theme.color("surface0"));
                }
                frame.render_widget(
                    Paragraph::new(format!("  {query}"))
                        .style(Style::default().fg(theme.color("subtext1"))),
                    rect,
                );
                register(hits, rect, action);
                y += 1;
            }
        }
        return;
    }
    if app.search_loading {
        draw_center_message(
            frame,
            theme,
            area,
            "人家正在找仓库喵~...",
            "搜索结果一会儿就回来啦喵~",
        );
        return;
    }
    if app.search_results.is_empty() {
        draw_center_message(
            frame,
            theme,
            area,
            "没有搜到仓库喵~",
            "换个关键词，或者去掉一些限定条件试试嘛喵~",
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_repo_card(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    repo: &Repo,
    selected: bool,
    hovered: bool,
    owner_hovered: bool,
    pinned: bool,
    hits: &mut Vec<HitBox>,
) {
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
    }
    if hovered && !selected {
        fill(
            frame,
            Rect::new(area.x, area.y, 1, area.height),
            theme.color("surface2"),
        );
    }
    let left = area.x + 2;
    let width = area.width.saturating_sub(4);
    let owner = repo.owner_name();
    let owner_style = if owner_hovered {
        Style::default()
            .fg(theme.color("blue"))
            .add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default().fg(theme.color("overlay1"))
    };
    let mut title = vec![
        Span::styled(
            format!("{} ", icons::REPO),
            Style::default().fg(theme.color("blue")),
        ),
        Span::styled(owner.to_string(), owner_style),
        Span::styled("/", Style::default().fg(theme.color("overlay0"))),
        Span::styled(
            repo.name.clone(),
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if repo.archived {
        title.push(Span::styled(
            format!("  {} 归档", icons::ARCHIVE),
            Style::default().fg(theme.color("peach")),
        ));
    }
    if pinned {
        title.push(Span::styled(
            format!("  {} 置顶", icons::PIN),
            Style::default().fg(theme.color("mauve")),
        ));
    }
    let (visibility_icon, visibility_text) = if repo.private {
        (icons::LOCK, "私有喵~")
    } else {
        (icons::GLOBE, "公开喵~")
    };
    let right = format!(
        "{} {}   {} {}",
        visibility_icon,
        visibility_text,
        icons::CLOCK,
        format::relative_time(repo.updated_at())
    );
    let right_width = format::text_width(&right) as u16 + 1;
    frame.render_widget(
        Paragraph::new(Line::from(title)),
        Rect::new(left, area.y, width.saturating_sub(right_width), 1),
    );
    let title_width = width.saturating_sub(right_width);
    register_owner_hit(hits, left, area.y, title_width, owner);
    frame.render_widget(
        Paragraph::new(format::truncate(&right, width as usize))
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(left, area.y, width, 1),
    );

    let description =
        format::truncate(&format::clean_text(repo.description_text()), width as usize);
    frame.render_widget(
        Paragraph::new(description).style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(left, area.y + 1, width, 1),
    );

    let mut meta = vec![
        Span::styled(
            repo.language_label().to_string(),
            Style::default().fg(language_color(theme, repo.language_label())),
        ),
        Span::styled(
            format!("   {} ", icons::STAR),
            Style::default().fg(theme.color("overlay0")),
        ),
        Span::styled(
            format::human_count(repo.stargazers_count),
            Style::default().fg(theme.color("subtext1")),
        ),
        Span::styled(
            format!("   {} ", icons::FORK),
            Style::default().fg(theme.color("overlay0")),
        ),
        Span::styled(
            format::human_count(repo.forks_count),
            Style::default().fg(theme.color("subtext1")),
        ),
        Span::styled(
            format!("   {} ", icons::ISSUE),
            Style::default().fg(theme.color("overlay0")),
        ),
        Span::styled(
            format::human_count(repo.open_issues_count),
            Style::default().fg(theme.color("subtext1")),
        ),
    ];
    if repo.private {
        meta.push(Span::styled(
            format!("   {} 私有", icons::LOCK),
            Style::default().fg(theme.color("mauve")),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(meta)),
        Rect::new(left, area.y + 2, width, 1),
    );

    if !repo.topics.is_empty() {
        let topics = repo
            .topics
            .iter()
            .take(5)
            .map(|topic| format!("#{}", format::clean_text(topic)))
            .collect::<Vec<_>>()
            .join("  ");
        frame.render_widget(
            Paragraph::new(format::truncate(&topics, width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(left, area.y + 3, width, 1),
        );
    }
}

fn repo_title_line(
    theme: &Theme,
    repo: &Repo,
    owner_hovered: bool,
    max_width: u16,
) -> Line<'static> {
    let icon = format!("{} ", icons::REPO);
    let owner = repo.owner_name();
    let name = repo.name.as_str();
    let icon_width = format::text_width(&icon);
    let max_width = max_width as usize;
    let name_style = Style::default()
        .fg(theme.color("text"))
        .add_modifier(Modifier::BOLD);
    if max_width <= icon_width {
        return Line::from(Span::styled(
            format::truncate(&format!("{icon}{owner}/{name}"), max_width),
            name_style,
        ));
    }
    let owner_style = if owner_hovered {
        Style::default()
            .fg(theme.color("blue"))
            .add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default().fg(theme.color("overlay1"))
    };
    let owner_display = format::truncate(owner, max_width - icon_width);
    let owner_width = format::text_width(&owner_display);
    let mut spans = vec![
        Span::styled(icon, name_style),
        Span::styled(owner_display, owner_style),
    ];
    let used = icon_width + owner_width;
    if used < max_width {
        spans.push(Span::styled(
            "/",
            Style::default().fg(theme.color("overlay0")),
        ));
        if used + 1 < max_width {
            spans.push(Span::styled(
                format::truncate(name, max_width - used - 1),
                name_style,
            ));
        }
    }
    Line::from(spans)
}

fn draw_repository_detail(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let Some(selected) = app.selected_repo() else {
        draw_center_message(
            frame,
            theme,
            area,
            "挑一个仓库看看喵~",
            "点左边卡片，人家就把详情摆在这里喵~",
        );
        return;
    };
    let repo = app
        .detail
        .clone()
        .filter(|detail| detail.display_full_name() == selected.display_full_name())
        .unwrap_or(selected);
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 8 {
        return;
    }
    frame.render_widget(
        Paragraph::new(icons::label(icons::REPO, "仓库小档案喵~"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let owner_hovered = app
        .hover
        .as_ref()
        .is_some_and(|action| action == &Action::OpenProfile(repo.owner_name().to_string()));
    frame.render_widget(
        Paragraph::new(repo_title_line(theme, &repo, owner_hovered, inner.width)),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    register_owner_hit(hits, inner.x, inner.y + 1, inner.width, repo.owner_name());
    let mut y = inner.y + 2;
    let description = format::wrap_text(
        &format::clean_text(repo.description_text()),
        inner.width as usize,
        3,
    );
    for line in description {
        frame.render_widget(
            Paragraph::new(line).style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
    if y > inner.y + 5 {
        y = inner.y + 5;
    }

    let visibility_icon = if repo.private {
        icons::LOCK
    } else {
        icons::GLOBE
    };
    let badges = format!(
        "{visibility_icon} {}   {} {}   {} {}",
        repo.visibility(),
        icons::CODE,
        repo.language_label(),
        icons::LICENSE,
        repo.license_label()
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&badges, inner.width as usize))
            .style(Style::default().fg(theme.color("sapphire"))),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 2;
    if y + 3 < inner.y + inner.height {
        let stats = [
            (
                format!("{} 星标喵~", icons::STAR),
                format::human_count(repo.stargazers_count),
                format!("{} 复刻喵~", icons::FORK),
                format::human_count(repo.forks_count),
            ),
            (
                format!("{} 议题喵~", icons::ISSUE),
                format::human_count(repo.open_issues_count),
                format!("{} 关注喵~", icons::EYE),
                format::human_count(repo.watchers_count),
            ),
            (
                format!("{} 更新喵~", icons::CLOCK),
                format::relative_time(repo.updated_at()),
                format!("{} 分支喵~", icons::BRANCH),
                repo.default_branch.clone(),
            ),
        ];
        for (left_label, left_value, right_label, right_value) in stats {
            let line = Line::from(vec![
                Span::styled(
                    format!("{left_label} "),
                    Style::default().fg(theme.color("overlay0")),
                ),
                Span::styled(
                    format::truncate(&left_value, 10),
                    Style::default().fg(theme.color("text")),
                ),
                Span::styled(
                    format!("   {right_label} "),
                    Style::default().fg(theme.color("overlay0")),
                ),
                Span::styled(
                    format::truncate(&right_value, 12),
                    Style::default().fg(theme.color("text")),
                ),
            ]);
            frame.render_widget(Paragraph::new(line), Rect::new(inner.x, y, inner.width, 1));
            y += 1;
        }
    }

    let buttons_y = y + 1;
    if buttons_y < inner.y + inner.height {
        let mut x = inner.x;
        let mut buttons = vec![
            (
                icons::label(icons::EXTERNAL, "去主页喵~"),
                Action::OpenCurrent,
                false,
            ),
            (
                icons::label(icons::COPY, "复制喵~"),
                Action::CopyClone,
                false,
            ),
            (icons::label(icons::TAG, "打开喵~"), Action::OpenPage, true),
        ];
        if app.is_own_repo(&repo) {
            buttons.push((
                icons::label(icons::COGS, "编辑喵~"),
                Action::OpenRepoEdit,
                false,
            ));
        }
        for (label, action, primary) in buttons {
            let width = format::text_width(&label) as u16 + 4;
            if x + width > inner.x + inner.width {
                break;
            }
            draw_button(
                frame,
                app,
                theme,
                hits,
                Rect::new(x, buttons_y, width, 1),
                &label,
                action,
                primary,
            );
            x += width + 1;
        }
    }

    let releases_y = buttons_y + 2;
    if releases_y >= inner.y + inner.height {
        return;
    }
    fill(
        frame,
        Rect::new(inner.x, releases_y, inner.width, 1),
        theme.color("surface0"),
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::TAG, "发行版喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, releases_y + 1, inner.width, 1),
    );
    let loading_note = app
        .releases_loading
        .as_ref()
        .is_some_and(|name| name == &repo.display_full_name());
    if loading_note {
        frame.render_widget(
            Paragraph::new(format!("{} 人家正在取喵~...", app.spinner()))
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(inner.x, releases_y + 2, inner.width, 1),
        );
        return;
    }
    if app.releases.is_empty() {
        frame.render_widget(
            Paragraph::new("这个仓库还没有发行版呢喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, releases_y + 2, inner.width, 1),
        );
        return;
    }
    let release_area = Rect::new(
        inner.x,
        releases_y + 2,
        inner.width,
        (inner.y + inner.height).saturating_sub(releases_y + 2),
    );
    let mut lines: Vec<Line> = Vec::new();
    for release in &app.releases {
        lines.push(Line::from(vec![
            Span::styled(
                release.tag_name.clone(),
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", format::relative_time(release.date())),
                Style::default().fg(theme.color("overlay0")),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            format::truncate(
                &format::clean_text(release.title()),
                release_area.width as usize,
            ),
            Style::default().fg(theme.color("subtext0")),
        )));
        lines.push(Line::from(Span::styled(
            format!("资产 {} 个喵~", release.assets.len()),
            Style::default().fg(theme.color("overlay0")),
        )));
        lines.push(Line::default());
    }
    app.detail_scroll_max = lines.len().saturating_sub(release_area.height as usize) as u16;
    app.detail_scroll = app.detail_scroll.min(app.detail_scroll_max);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((app.detail_scroll, 0)),
        release_area,
    );
    if app.client.proxy().contains("v4.gh-proxy.org") {
        frame.render_widget(
            Paragraph::new("发行版资产走 v4 加速通道喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                inner.x,
                inner.y + inner.height.saturating_sub(1),
                inner.width,
                1,
            ),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_page_title(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    inner: Rect,
    title: &str,
    subtitle: &str,
    refresh_label: &str,
    refresh_action: Action,
    hits: &mut Vec<HitBox>,
) {
    frame.render_widget(
        Paragraph::new(title).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width.saturating_sub(12), 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(subtitle, inner.width as usize))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let width = format::text_width(refresh_label) as u16 + 4;
    if inner.width > width + 4 {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(inner.x + inner.width - width, inner.y, width, 1),
            refresh_label,
            refresh_action,
            false,
        );
    }
}

fn draw_sort_chips(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    area: Rect,
    label: &str,
    chips: &[(&str, Action, bool)],
) {
    let label_width = format::text_width(label) as u16;
    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x, area.y, label_width + 1, 1),
    );
    let mut x = area.x + label_width + 2;
    for (text, action, active) in chips {
        let width = format::text_width(text) as u16 + 3;
        if x + width > area.x + area.width {
            break;
        }
        let rect = Rect::new(x, area.y, width, 1);
        let hovered = app.hover.as_ref() == Some(action);
        let bg = if *active {
            theme.color("surface1")
        } else if hovered {
            theme.color("surface0")
        } else {
            theme.color("base")
        };
        fill(frame, rect, bg);
        let style = if *active {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("subtext0"))
        };
        frame.render_widget(
            Paragraph::new(format!(" {text}")).style(style),
            Rect::new(rect.x, rect.y, rect.width, 1),
        );
        register(hits, rect, action.clone());
        x += width + 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_repo_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let Some(base_repo) = app.page_repo.clone() else {
        draw_center_message(
            frame,
            theme,
            area,
            "仓库不见啦喵~",
            "按 Esc 回列表再选一次嘛喵~",
        );
        return;
    };
    let repo = app
        .detail
        .clone()
        .filter(|detail| detail.display_full_name() == base_repo.display_full_name())
        .unwrap_or(base_repo);
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 12 {
        draw_center_message(
            frame,
            theme,
            inner,
            "这里有点挤喵~",
            "把终端拉大一点，人家才摆得下仓库页喵~",
        );
        return;
    }

    if app.readme_fullscreen {
        app.layout.page_readme = inner;
        app.layout.page_tree = Rect::default();
        app.layout.page_releases = Rect::default();
        draw_readme_panel(frame, app, theme, inner, hits);
        return;
    }

    let back_label = icons::label(icons::BACK, "返回喵~");
    let back_width = format::text_width(&back_label) as u16 + 2;
    let back_rect = Rect::new(inner.x, inner.y, back_width, 1);
    let back_hover = app.hover.as_ref() == Some(&Action::Back);
    frame.render_widget(
        Paragraph::new(back_label).style(if back_hover {
            Style::default().fg(theme.color("blue"))
        } else {
            Style::default().fg(theme.color("subtext0"))
        }),
        back_rect,
    );
    register(hits, back_rect, Action::Back);
    let crumb = format!(
        "{} / {} / {}",
        app.return_page.title(),
        repo.owner_name(),
        repo.name
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &crumb,
            inner.width.saturating_sub(back_width + 14) as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            inner.x + back_width + 2,
            inner.y,
            inner.width.saturating_sub(back_width + 14),
            1,
        ),
    );
    let refresh_label = icons::label(icons::REFRESH, "刷新一下喵~");
    let refresh_width = format::text_width(&refresh_label) as u16 + 4;
    let refresh_x = inner.x + inner.width.saturating_sub(refresh_width);
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(refresh_x, inner.y, refresh_width, 1),
        &refresh_label,
        Action::Refresh,
        false,
    );
    if app.is_own_repo(&repo) {
        let edit_label = icons::label(icons::COGS, "编辑仓库喵~");
        let edit_width = format::text_width(&edit_label) as u16 + 4;
        let edit_x = refresh_x.saturating_sub(edit_width + 1);
        if edit_x > inner.x + back_width {
            draw_button(
                frame,
                app,
                theme,
                hits,
                Rect::new(edit_x, inner.y, edit_width, 1),
                &edit_label,
                Action::OpenRepoEdit,
                true,
            );
        }
    }

    let starred = app.is_current_starred();
    let star_label = if app.star_loading.is_some() {
        format!("{} 处理中喵~", icons::STAR)
    } else if starred {
        format!("{} 已收藏喵~", icons::STAR)
    } else {
        format!("{} 收藏喵~", icons::STAR_EMPTY)
    };
    let star_width = format::text_width(&star_label) as u16 + 4;
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(
            inner.x + inner.width.saturating_sub(star_width),
            inner.y + 1,
            star_width,
            1,
        ),
        &star_label,
        Action::ToggleStar,
        !starred,
    );

    let title_width = inner.width.saturating_sub(star_width + 3);
    let owner_hovered = app
        .hover
        .as_ref()
        .is_some_and(|action| action == &Action::OpenProfile(repo.owner_name().to_string()));
    frame.render_widget(
        Paragraph::new(repo_title_line(theme, &repo, owner_hovered, title_width)),
        Rect::new(inner.x, inner.y + 1, title_width, 1),
    );
    register_owner_hit(hits, inner.x, inner.y + 1, title_width, repo.owner_name());
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format::clean_text(repo.description_text()),
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );
    let visibility_icon = if repo.private {
        icons::LOCK
    } else {
        icons::GLOBE
    };
    let mut facts = vec![
        format!("{visibility_icon} {}", repo.visibility()),
        format!("{} {}", icons::BRANCH, app.branch_for(&repo)),
        format!("{} {}", icons::LICENSE, repo.license_label()),
        format!(
            "{} {}",
            icons::DATABASE,
            format::format_size(repo.size.saturating_mul(1024))
        ),
    ];
    for topic in repo.topics.iter().take(3) {
        facts.push(format!("#{}", format::clean_text(topic)));
    }
    frame.render_widget(
        Paragraph::new(format::truncate(&facts.join("   "), inner.width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
    );
    let stats = format!(
        "{} {}   {} {}   {} {}   {} {}   {} {}",
        icons::STAR,
        format::human_count(repo.stargazers_count),
        icons::FORK,
        format::human_count(repo.forks_count),
        icons::EYE,
        format::human_count(repo.watchers_count),
        icons::ISSUE,
        format::human_count(repo.open_issues_count),
        icons::CLOCK,
        format::relative_time(repo.updated_at())
    );
    frame.render_widget(
        Paragraph::new(stats).style(Style::default().fg(theme.color("text"))),
        Rect::new(inner.x, inner.y + 4, inner.width, 1),
    );

    let actions_y = inner.y + 5;
    let mut action_x = inner.x;
    let mut buttons = vec![
        (
            icons::label(icons::EXTERNAL, "去主页喵~"),
            Action::OpenCurrent,
            false,
        ),
        (
            icons::label(icons::COPY, "复制克隆喵~"),
            Action::CopyClone,
            false,
        ),
        (
            icons::label(icons::DOWNLOAD, "下载源码喵~"),
            Action::DownloadSource,
            false,
        ),
        (
            icons::label(icons::BRANCH, "分支喵~"),
            Action::OpenBranches,
            false,
        ),
        (
            icons::label(icons::COMMIT, "提交喵~"),
            Action::OpenCommits,
            false,
        ),
        (
            icons::label(icons::CODE, "语言喵~"),
            Action::OpenRepoLanguages,
            false,
        ),
        (
            icons::label(icons::ISSUE, "议题喵~"),
            Action::OpenIssues,
            false,
        ),
        (
            icons::label(icons::PULL, "合并请求喵~"),
            Action::OpenPullRequests,
            false,
        ),
        (
            icons::label(icons::COGS, "工作流喵~"),
            Action::OpenActions,
            false,
        ),
    ];
    if !app.is_own_repo(&repo) {
        buttons.insert(
            2,
            (
                icons::label(icons::FORK, "Fork 喵~"),
                Action::ForkRepo,
                false,
            ),
        );
    } else {
        buttons.insert(
            2,
            (
                icons::label(icons::TRASH, "删除喵~"),
                Action::DeleteRepo,
                false,
            ),
        );
    }
    for (label, action, primary) in buttons {
        let button_width = format::text_width(&label) as u16 + 4;
        if action_x + button_width > inner.x + inner.width {
            break;
        }
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(action_x, actions_y, button_width, 1),
            &label,
            action,
            primary,
        );
        action_x += button_width + 1;
    }
    let divider_y = inner.y + 6;
    fill(
        frame,
        Rect::new(inner.x, divider_y, inner.width, 1),
        theme.color("surface0"),
    );
    let content = Rect::new(
        inner.x,
        divider_y + 1,
        inner.width,
        (inner.y + inner.height).saturating_sub(divider_y + 1),
    );
    if content.height < 4 {
        return;
    }

    let show_readme = app.config.show_readme && content.height >= 9;
    let readme_height = (content.height * 2 / 5).max(4);
    let (main_area, readme_area) = if show_readme {
        let split = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(readme_height.min(content.height.saturating_sub(4))),
        ])
        .split(content);
        (split[0], split[1])
    } else {
        (content, Rect::default())
    };
    let columns = Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(main_area);
    let tree_rect = columns[0];
    let releases_rect = columns[1];
    app.layout.page_tree = tree_rect;
    app.layout.page_releases = releases_rect;
    app.layout.page_readme = readme_area;
    draw_file_tree_panel(frame, app, theme, &repo, tree_rect, hits);
    draw_releases_panel(frame, app, theme, &repo, releases_rect, hits);
    if show_readme {
        draw_readme_panel(frame, app, theme, readme_area, hits);
    } else {
        app.readme_scroll = 0;
        app.readme_scroll_max = 0;
    }
}

fn workflow_status_icon(status: &str, conclusion: Option<&str>) -> (&'static str, &'static str) {
    match conclusion {
        Some("success") => (icons::CHECK, "green"),
        Some("failure" | "timed_out" | "cancelled" | "startup_failure" | "action_required") => {
            (icons::WARNING, "red")
        }
        Some("skipped" | "neutral" | "stale") => (icons::INFO, "overlay0"),
        _ => match status {
            "in_progress" => (icons::CLOCK, "peach"),
            "queued" | "waiting" | "requested" | "pending" => (icons::CLOCK, "overlay0"),
            _ => (icons::INFO, "blue"),
        },
    }
}

fn release_tag_pill(theme: &Theme, tag: &str, accent: Color) -> Span<'static> {
    Span::styled(
        format!(" {tag} "),
        Style::default()
            .bg(accent)
            .fg(theme.color("base"))
            .add_modifier(Modifier::BOLD),
    )
}

fn release_badge(icon: &str, text: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!("  {icon} {text}"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn draw_release_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let Some(base_repo) = app.page_repo.clone() else {
        draw_center_message(
            frame,
            theme,
            area,
            "仓库不见啦喵~",
            "按 Esc 回列表再选一次嘛喵~",
        );
        return;
    };
    let Some(release) = app.selected_release() else {
        draw_center_message(
            frame,
            theme,
            area,
            "发行版不见啦喵~",
            "按 Esc 回仓库页刷新一下嘛喵~",
        );
        return;
    };
    let repo = app
        .detail
        .clone()
        .filter(|detail| detail.display_full_name() == base_repo.display_full_name())
        .unwrap_or(base_repo);
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 10 {
        draw_center_message(
            frame,
            theme,
            inner,
            "这里有点挤喵~",
            "把终端拉大一点，人家才摆得下发行版页喵~",
        );
        return;
    }

    let back_label = icons::label(icons::BACK, "返回喵~");
    let back_width = format::text_width(&back_label) as u16 + 2;
    let back_rect = Rect::new(inner.x, inner.y, back_width, 1);
    let back_hover = app.hover.as_ref() == Some(&Action::Back);
    frame.render_widget(
        Paragraph::new(back_label).style(if back_hover {
            Style::default().fg(theme.color("blue"))
        } else {
            Style::default().fg(theme.color("subtext0"))
        }),
        back_rect,
    );
    register(hits, back_rect, Action::Back);

    let refresh_label = icons::label(icons::REFRESH, "刷新喵~");
    let refresh_width = format::text_width(&refresh_label) as u16 + 4;
    let refresh_x = inner.x + inner.width.saturating_sub(refresh_width);
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(refresh_x, inner.y, refresh_width, 1),
        &refresh_label,
        Action::Refresh,
        false,
    );
    let title = format!("{} 发行版喵~  {}", icons::TAG, repo.display_full_name());
    let title_width = refresh_x.saturating_sub(inner.x + back_width + 2);
    frame.render_widget(
        Paragraph::new(format::truncate(&title, title_width as usize)).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x + back_width + 2, inner.y, title_width, 1),
    );

    let accent = if release.prerelease {
        theme.color("mauve")
    } else if release.draft {
        theme.color("red")
    } else {
        theme.color("blue")
    };
    let tag_text = format::truncate(&release.tag_name, (inner.width / 3).max(6) as usize);
    let pill = release_tag_pill(theme, &tag_text, accent);
    let tag_width = format::text_width(&tag_text) as u16 + 2;
    let mut badges = vec![
        pill,
        Span::styled("  ", Style::default()),
        Span::styled(
            format::truncate(
                release.title(),
                inner.width.saturating_sub(tag_width + 18) as usize,
            ),
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if app.release_selected == 0 {
        badges.push(release_badge(icons::SPARKLE, "最新", theme.color("green")));
    }
    if release.prerelease {
        badges.push(release_badge(icons::CLOCK, "预发布", theme.color("peach")));
    }
    if release.draft {
        badges.push(release_badge(icons::WARNING, "草稿", theme.color("red")));
    }
    frame.render_widget(
        Paragraph::new(Line::from(badges)),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{}  ·  {} 个附带文件喵~",
            format::relative_time(release.date()),
            release.assets.len()
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );

    let mut button_x = inner.x;
    let open_label = icons::label(icons::EXTERNAL, "去 GitHub喵~");
    let open_width = format::text_width(&open_label) as u16 + 4;
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(button_x, inner.y + 3, open_width, 1),
        &open_label,
        Action::OpenReleasePage(app.release_selected),
        false,
    );
    button_x += open_width + 1;
    let source_label = icons::label(icons::DOWNLOAD, "下载源码喵~");
    let source_width = format::text_width(&source_label) as u16 + 4;
    if button_x + source_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(button_x, inner.y + 3, source_width, 1),
            &source_label,
            Action::DownloadSource,
            false,
        );
    }
    fill(
        frame,
        Rect::new(inner.x, inner.y + 4, inner.width, 1),
        theme.color("surface0"),
    );

    let content = Rect::new(
        inner.x,
        inner.y + 5,
        inner.width,
        inner.height.saturating_sub(5),
    );
    if content.height < 3 {
        return;
    }
    let left_width = if content.width >= 90 {
        content.width * 3 / 5
    } else {
        content.width * 11 / 20
    };
    let left = Rect::new(content.x, content.y, left_width, content.height);
    let right = Rect::new(
        content.x + left_width + 2,
        content.y,
        content.width.saturating_sub(left_width + 2),
        content.height,
    );

    app.layout.detail = left;
    frame.render_widget(
        Paragraph::new(icons::label(icons::BOOK, "发行说明喵~")).style(
            Style::default()
                .fg(theme.color("overlay0"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(left.x, left.y, left.width, 1),
    );
    let body_rect = Rect::new(
        left.x,
        left.y + 2,
        left.width,
        left.height.saturating_sub(2),
    );
    let body_text = release
        .body
        .as_deref()
        .map(str::trim)
        .filter(|body| !body.is_empty())
        .unwrap_or("这个发行版没有写说明喵~");
    let mut lines: Vec<Line> = Vec::new();
    for raw in body_text.lines().take(400) {
        let cleaned = strip_markdown_inline(raw.trim());
        if cleaned.trim().is_empty() {
            lines.push(Line::default());
        } else {
            for part in format::wrap_text(&cleaned, body_rect.width as usize, 1_000) {
                lines.push(Line::from(Span::styled(
                    part,
                    Style::default().fg(theme.color("subtext0")),
                )));
            }
        }
    }
    app.release_scroll_max = lines.len().saturating_sub(body_rect.height as usize) as u16;
    app.release_scroll = app.release_scroll.min(app.release_scroll_max);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((app.release_scroll, 0)),
        body_rect,
    );

    if right.width < 10 {
        return;
    }
    frame.render_widget(
        Paragraph::new(icons::label(icons::DOWNLOAD, "附带文件喵~")).style(
            Style::default()
                .fg(theme.color("overlay0"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(right.x, right.y, right.width, 1),
    );
    let assets_rect = Rect::new(
        right.x,
        right.y + 2,
        right.width,
        right.height.saturating_sub(3),
    );
    app.layout.page_releases = assets_rect;
    if release.assets.is_empty() {
        draw_center_message(
            frame,
            theme,
            assets_rect,
            "没有附带文件喵~",
            "这个发行版只有源码包喵~",
        );
        return;
    }
    let rows = assets_rect.height as usize;
    let max_start = release.assets.len().saturating_sub(rows);
    let start = app
        .release_asset_selected
        .saturating_sub(rows.saturating_sub(1));
    let start = start.min(max_start);
    let end = (start + rows).min(release.assets.len());
    for (offset, index) in (start..end).enumerate() {
        let asset = &release.assets[index];
        let y = assets_rect.y + offset as u16;
        let rect = Rect::new(assets_rect.x, y, assets_rect.width, 1);
        let action = Action::DownloadAsset {
            release: app.release_selected,
            asset: index,
        };
        let selected = index == app.release_asset_selected;
        let hovered = app.hover.as_ref() == Some(&action);
        fill(
            frame,
            rect,
            if selected || hovered {
                theme.color("surface1")
            } else {
                theme.color("surface0")
            },
        );
        if selected {
            fill(frame, Rect::new(rect.x, rect.y, 1, 1), theme.color("blue"));
        }
        let name_width = rect.width.saturating_sub(15) as usize;
        let name = format::truncate(&format::clean_text(&asset.name), name_width);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{} ", icons::DOWNLOAD),
                    Style::default().fg(theme.color("blue")),
                ),
                Span::styled(name, Style::default().fg(theme.color("text"))),
            ])),
            Rect::new(rect.x + 2, y, rect.width.saturating_sub(2), 1),
        );
        frame.render_widget(
            Paragraph::new(format::format_size(asset.size))
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(rect.x, y, rect.width.saturating_sub(1), 1),
        );
        register(hits, rect, action);
    }
    frame.render_widget(
        Paragraph::new("Enter 或点文件下载到上面目录喵~")
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            right.x,
            right.y + right.height.saturating_sub(1),
            right.width,
            1,
        ),
    );
}

fn draw_notifications_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 8 {
        draw_center_message(
            frame,
            theme,
            inner,
            "这里有点挤喵~",
            "把终端拉大一点，人家才摆得下通知页喵~",
        );
        return;
    }

    draw_page_title(
        frame,
        app,
        theme,
        inner,
        &icons::label(icons::BELL, "通知喵~"),
        "GitHub 的通知都收在这里喵~",
        "刷新喵~",
        Action::Refresh,
        hits,
    );

    let unread = app.notifications.iter().filter(|item| item.unread).count();
    let summary = format!(
        "当前 {} 条 · 未读 {} 条 · {}喵~",
        app.notifications.len(),
        unread,
        if app.notifications_all {
            "看全部"
        } else {
            "只看未读"
        }
    );
    let clear_label = icons::label(icons::CHECK_CIRCLE, "全部已读喵~");
    let clear_width = format::text_width(&clear_label) as u16 + 4;
    let summary_width = inner.width.saturating_sub(clear_width + 2);
    frame.render_widget(
        Paragraph::new(format::truncate(&summary, summary_width as usize))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 2, summary_width, 1),
    );
    if inner.width > clear_width + 8 {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(
                inner.x + inner.width.saturating_sub(clear_width),
                inner.y + 2,
                clear_width,
                1,
            ),
            &clear_label,
            Action::MarkAllNotificationsRead,
            false,
        );
    }

    draw_sort_chips(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
        &icons::label(icons::BELL, "筛选喵~"),
        &[
            (
                "只看未读喵~",
                Action::SetNotificationFilter(false),
                !app.notifications_all,
            ),
            (
                "全部通知喵~",
                Action::SetNotificationFilter(true),
                app.notifications_all,
            ),
        ],
    );

    let content = Rect::new(
        inner.x,
        inner.y + 5,
        inner.width,
        inner.height.saturating_sub(5),
    );
    if content.height < 4 {
        return;
    }
    let detail_width = if content.width >= 96 {
        (content.width * 2 / 5).max(32)
    } else {
        0
    };
    let columns =
        Layout::horizontal([Constraint::Min(32), Constraint::Length(detail_width)]).split(content);
    app.layout.list = columns[0];
    draw_notification_list(frame, app, theme, columns[0], hits);
    if detail_width > 0 {
        app.layout.detail = columns[1];
        draw_notification_detail(frame, app, theme, columns[1], hits);
    }
}

fn draw_notification_list(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Paragraph::new(icons::label(icons::BELL, "通知列表喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    frame.render_widget(
        Paragraph::new(format!("{} 条喵~", app.notifications.len()))
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
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
    if app.notifications.is_empty() {
        let (title, detail) = if app.notifications_loading.is_some() {
            ("人家正在收通知喵~...", "第一次同步要等一小会儿哦喵~")
        } else if !app.client.token_present() {
            (
                "还没有登录 GitHub喵~",
                "通知要登录才看得到，先跑 gh auth login 嘛喵~",
            )
        } else if app.notifications_all {
            ("这里还没有通知喵~", "GitHub 那边很安静喵~")
        } else {
            (
                "没有未读通知啦喵~",
                "按 u 看全部通知，或者去 GitHub 逛逛喵~",
            )
        };
        draw_center_message(frame, theme, body, title, detail);
        return;
    }

    let rows = (body.height / 2).max(1) as usize;
    let selected = app.notification_selected.min(app.notifications.len() - 1);
    let start = selected.saturating_sub(rows - 1);
    let end = (start + rows).min(app.notifications.len());
    for (offset, index) in (start..end).enumerate() {
        let y = body.y + offset as u16 * 2;
        if y + 1 >= body.y + body.height {
            break;
        }
        let rect = Rect::new(body.x, y, body.width, 2);
        if index == selected {
            fill(frame, rect, theme.color("surface0"));
        }
        let item = &app.notifications[index];
        let (icon, color) = if item.unread {
            (icons::BELL, "blue")
        } else {
            (icons::CHECK, "overlay0")
        };
        let title_width = body.width.saturating_sub(4) as usize;
        let title = format!(
            "{} {}",
            item.subject_type_label(),
            format::clean_text(&item.subject.title)
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{icon} "), Style::default().fg(theme.color(color))),
                Span::styled(
                    format::truncate(&title, title_width),
                    Style::default().fg(theme.color("text")),
                ),
            ])),
            Rect::new(body.x + 1, y, body.width.saturating_sub(1), 1),
        );
        let meta = format!(
            "   {} · {} · {}",
            item.repo_label(),
            item.reason_label(),
            format::relative_time(&item.updated_at)
        );
        frame.render_widget(
            Paragraph::new(format::truncate(&meta, body.width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(body.x + 1, y + 1, body.width.saturating_sub(1), 1),
        );
        register(hits, rect, Action::SelectNotification(index));
    }
}

fn draw_notification_detail(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.width < 10 || inner.height < 5 {
        return;
    }
    let Some(item) = app.selected_notification() else {
        draw_center_message(
            frame,
            theme,
            area,
            "挑一条通知看看喵~",
            "点左边列表，人家就把通知摊开给你看喵~",
        );
        return;
    };
    frame.render_widget(
        Paragraph::new(icons::label(icons::BELL, "通知小档案喵~")).style(
            Style::default()
                .fg(theme.color("overlay0"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let title = format::clean_text(&item.subject.title);
    let mut y = inner.y + 2;
    for line in format::wrap_text(&title, inner.width as usize, 3) {
        frame.render_widget(
            Paragraph::new(line).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
    y += 1;
    let facts = [
        ("类型喵~", item.subject_type_label().to_string()),
        ("原因喵~", item.reason_label().to_string()),
        ("仓库喵~", item.repo_label()),
        ("更新喵~", format::relative_time(&item.updated_at)),
        (
            "状态喵~",
            if item.unread {
                "未读喵~".to_string()
            } else {
                "已读喵~".to_string()
            },
        ),
    ];
    for (label, value) in facts {
        if y + 2 >= inner.y + inner.height {
            break;
        }
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{label}  "),
                    Style::default().fg(theme.color("overlay0")),
                ),
                Span::styled(
                    format::truncate(&value, inner.width.saturating_sub(8) as usize),
                    Style::default().fg(theme.color("subtext0")),
                ),
            ])),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
    let mark = icons::label(icons::CHECK, "标成已读喵~");
    let mark_width = format::text_width(&mark) as u16 + 4;
    if y + 3 < inner.y + inner.height && item.unread {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(inner.x, y + 1, mark_width, 1),
            &mark,
            Action::MarkNotificationRead,
            true,
        );
    }
    frame.render_widget(
        Paragraph::new("Enter / o 去 GitHub 看完整内容喵~")
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(2),
            inner.width,
            1,
        ),
    );
}

fn draw_issues_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 10 {
        draw_center_message(
            frame,
            theme,
            inner,
            "这里有点挤喵~",
            "把终端拉大一点，人家才摆得下议题页喵~",
        );
        return;
    }

    let back_label = icons::label(icons::BACK, "返回喵~");
    let back_width = format::text_width(&back_label) as u16 + 2;
    let back_rect = Rect::new(inner.x, inner.y, back_width, 1);
    let back_hover = app.hover.as_ref() == Some(&Action::Back);
    frame.render_widget(
        Paragraph::new(back_label).style(if back_hover {
            Style::default().fg(theme.color("blue"))
        } else {
            Style::default().fg(theme.color("subtext0"))
        }),
        back_rect,
    );
    register(hits, back_rect, Action::Back);

    let refresh_label = icons::label(icons::REFRESH, "刷新喵~");
    let refresh_width = format::text_width(&refresh_label) as u16 + 4;
    let refresh_x = inner.x + inner.width.saturating_sub(refresh_width);
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(refresh_x, inner.y, refresh_width, 1),
        &refresh_label,
        Action::Refresh,
        false,
    );
    let new_label = icons::label(icons::COMMENT, "新建议题喵~");
    let new_width = format::text_width(&new_label) as u16 + 4;
    let new_x = refresh_x.saturating_sub(new_width + 1);
    if new_x > inner.x + back_width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(new_x, inner.y, new_width, 1),
            &new_label,
            Action::StartNewIssue,
            true,
        );
    }
    let title = format!(
        "{} 议题与合并请求喵~  {}",
        icons::ISSUE,
        app.issue_repo_label()
    );
    let title_width = new_x.saturating_sub(inner.x + back_width + 2);
    frame.render_widget(
        Paragraph::new(format::truncate(&title, title_width as usize)).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x + back_width + 2, inner.y, title_width, 1),
    );
    frame.render_widget(
        Paragraph::new("这里把议题和合并请求放在一起，v 换类型，s 换状态喵~")
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    draw_sort_chips(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
        &icons::label(icons::ISSUE, "类型喵~"),
        &[
            (
                IssueKind::All.label(),
                Action::SetIssueKind(IssueKind::All),
                app.issue_kind == IssueKind::All,
            ),
            (
                IssueKind::Issues.label(),
                Action::SetIssueKind(IssueKind::Issues),
                app.issue_kind == IssueKind::Issues,
            ),
            (
                IssueKind::PullRequests.label(),
                Action::SetIssueKind(IssueKind::PullRequests),
                app.issue_kind == IssueKind::PullRequests,
            ),
        ],
    );
    draw_sort_chips(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
        &icons::label(icons::CHECK_CIRCLE, "状态喵~"),
        &[
            (
                IssueState::Open.label(),
                Action::SetIssueState(IssueState::Open),
                app.issue_state == IssueState::Open,
            ),
            (
                IssueState::Closed.label(),
                Action::SetIssueState(IssueState::Closed),
                app.issue_state == IssueState::Closed,
            ),
            (
                IssueState::All.label(),
                Action::SetIssueState(IssueState::All),
                app.issue_state == IssueState::All,
            ),
        ],
    );

    let content = Rect::new(
        inner.x,
        inner.y + 5,
        inner.width,
        inner.height.saturating_sub(5),
    );
    if content.height < 4 {
        return;
    }
    let detail_width = if content.width >= 94 {
        (content.width * 2 / 5).max(32)
    } else {
        0
    };
    let columns =
        Layout::horizontal([Constraint::Min(30), Constraint::Length(detail_width)]).split(content);
    app.layout.list = columns[0];
    draw_issue_list(frame, app, theme, columns[0], hits);
    if detail_width > 0 {
        app.layout.detail = columns[1];
        draw_issue_detail(frame, app, theme, columns[1], hits);
    }
}

fn draw_issue_list(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    let indices = app.issue_indices();
    frame.render_widget(
        Paragraph::new(icons::label(icons::ISSUE, "列表喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    frame.render_widget(
        Paragraph::new(format!("{} 条喵~", indices.len()))
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
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
    if indices.is_empty() {
        let (title, detail) = if app.issues_loading.is_some() {
            ("人家正在翻议题喵~...", "第一次同步要等一小会儿哦喵~")
        } else if app.issues.is_empty() {
            ("这个筛选下没有记录喵~", "按 s 换状态，或按 v 换类型看看喵~")
        } else {
            ("这个筛选下没有记录喵~", "换个筛选再找找嘛喵~")
        };
        draw_center_message(frame, theme, body, title, detail);
        return;
    }

    let rows = (body.height / 2).max(1) as usize;
    let selected = app.issue_selected.min(indices.len() - 1);
    let start = selected.saturating_sub(rows - 1);
    let end = (start + rows).min(indices.len());
    for (offset, position) in (start..end).enumerate() {
        let y = body.y + offset as u16 * 2;
        if y + 1 >= body.y + body.height {
            break;
        }
        let issue = &app.issues[indices[position]];
        let rect = Rect::new(body.x, y, body.width, 2);
        if position == selected {
            fill(frame, rect, theme.color("surface0"));
        }
        let (icon, color) = issue_status_icon(issue);
        let title_width = body.width.saturating_sub(7) as usize;
        let title = Line::from(vec![
            Span::styled(format!("{icon} "), Style::default().fg(theme.color(color))),
            Span::styled(
                format!("#{} ", issue.number),
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format::truncate(&format::clean_text(&issue.title), title_width),
                Style::default().fg(theme.color("text")),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(title),
            Rect::new(body.x + 1, y, body.width.saturating_sub(1), 1),
        );
        let author = issue.author();
        let labels = issue.labels_label();
        let meta = format!(
            "   {} · {} · {}{} · {}{}",
            issue.kind_label(),
            issue.state_label(),
            if author.is_empty() { "" } else { "by " },
            if author.is_empty() { "未知" } else { author },
            format::relative_time(&issue.updated_at),
            if labels.is_empty() {
                String::new()
            } else {
                format!(" · {labels}")
            }
        );
        frame.render_widget(
            Paragraph::new(format::truncate(&meta, body.width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(body.x + 1, y + 1, body.width.saturating_sub(1), 1),
        );
        register(hits, rect, Action::SelectIssue(position));
    }
}

fn draw_issue_detail(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.width < 10 || inner.height < 6 {
        return;
    }
    let Some(issue) = app.selected_issue() else {
        draw_center_message(
            frame,
            theme,
            area,
            "挑一条记录看看喵~",
            "点左边列表，人家就把正文摆在这里喵~",
        );
        return;
    };
    let (icon, color) = issue_status_icon(&issue);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{icon} "), Style::default().fg(theme.color(color))),
            Span::styled(
                format!("#{} ", issue.number),
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                issue.kind_label().to_string(),
                Style::default().fg(theme.color("text")),
            ),
        ])),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let mut y = inner.y + 1;
    for line in format::wrap_text(&format::clean_text(&issue.title), inner.width as usize, 3) {
        frame.render_widget(
            Paragraph::new(line).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
    if y < inner.y + inner.height {
        let author = issue.author();
        let meta = format!(
            "{} · {} · {}{} · {} {} 条评论",
            issue.state_label(),
            issue.kind_label(),
            if author.is_empty() { "" } else { "by " },
            if author.is_empty() { "未知" } else { author },
            icons::COMMENT,
            issue.comments
        );
        frame.render_widget(
            Paragraph::new(format::truncate(&meta, inner.width as usize))
                .style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
    if !issue.labels.is_empty() && y < inner.y + inner.height {
        let mut spans = vec![Span::styled(
            "标签喵~  ",
            Style::default().fg(theme.color("overlay0")),
        )];
        for label in issue.labels.iter().take(4) {
            spans.push(Span::styled(
                format!(" {} ", format::clean_text(&label.name)),
                Style::default()
                    .fg(issue_label_color(&label.color))
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" "));
        }
        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
    y += 1;
    let body_lines = (inner.y + inner.height).saturating_sub(y + 3) as usize;
    if body_lines >= 2 {
        let body = format::clean_text(issue.body_text());
        for line in format::wrap_text(&body, inner.width as usize, body_lines) {
            frame.render_widget(
                Paragraph::new(line).style(Style::default().fg(theme.color("subtext0"))),
                Rect::new(inner.x, y, inner.width, 1),
            );
            y += 1;
        }
    }
    let open = icons::label(icons::EXTERNAL, "去浏览器喵~");
    let open_width = format::text_width(&open) as u16 + 4;
    let button_y = inner.y + inner.height.saturating_sub(1);
    if y < button_y {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(inner.x, button_y, open_width, 1),
            &open,
            Action::OpenExternal(issue.html_url.clone()),
            true,
        );
    }
}

fn issue_status_icon(issue: &Issue) -> (&'static str, &'static str) {
    if issue.is_merged() {
        (icons::CHECK_CIRCLE, "blue")
    } else if issue.state == "open" {
        (icons::CHECK_CIRCLE, "green")
    } else if issue.is_pull_request() {
        (icons::X_CIRCLE, "red")
    } else {
        (icons::INFO, "overlay0")
    }
}

fn issue_label_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6
        && let Ok(value) = u32::from_str_radix(hex, 16)
    {
        return Color::Rgb(
            ((value >> 16) & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            (value & 0xff) as u8,
        );
    }
    Color::Gray
}

fn draw_commits_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 8 {
        draw_center_message(
            frame,
            theme,
            inner,
            "这里有点挤喵~",
            "把终端拉大一点，人家才摆得下提交页喵~",
        );
        return;
    }
    let back_label = icons::label(icons::BACK, "返回喵~");
    let back_width = format::text_width(&back_label) as u16 + 2;
    let back_rect = Rect::new(inner.x, inner.y, back_width, 1);
    frame.render_widget(
        Paragraph::new(back_label).style(if app.hover.as_ref() == Some(&Action::Back) {
            Style::default().fg(theme.color("blue"))
        } else {
            Style::default().fg(theme.color("subtext0"))
        }),
        back_rect,
    );
    register(hits, back_rect, Action::Back);
    let refresh_label = icons::label(icons::REFRESH, "刷新喵~");
    let refresh_width = format::text_width(&refresh_label) as u16 + 4;
    let refresh_x = inner.x + inner.width.saturating_sub(refresh_width);
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(refresh_x, inner.y, refresh_width, 1),
        &refresh_label,
        Action::Refresh,
        false,
    );
    let title_width = refresh_x.saturating_sub(inner.x + back_width + 2);
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("{} 提交喵~  {}", icons::COMMIT, app.commit_repo_label()),
            title_width as usize,
        ))
        .style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x + back_width + 2, inner.y, title_width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{} 个提交 · b 换分支 · Enter 去 GitHub 看详情喵~",
            app.commits.len()
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let content = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(3),
    );
    let detail_width = if content.width >= 94 {
        (content.width * 2 / 5).max(32)
    } else {
        0
    };
    let columns =
        Layout::horizontal([Constraint::Min(30), Constraint::Length(detail_width)]).split(content);
    app.layout.list = columns[0];
    draw_commit_list(frame, app, theme, columns[0], hits);
    if detail_width > 0 {
        app.layout.detail = columns[1];
        draw_commit_detail(frame, app, theme, columns[1], hits);
    }
}

fn draw_commit_list(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Paragraph::new(icons::label(icons::COMMIT, "提交列表喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
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
    if app.commits.is_empty() {
        let (title, detail) = if app.commits_loading.is_some() {
            ("人家正在翻提交记录喵~...", "网络慢一点就再等一会儿哦喵~")
        } else {
            ("这个分支还没有提交喵~", "按 b 换个分支，或按 r 刷新看看喵~")
        };
        draw_center_message(frame, theme, body, title, detail);
        return;
    }
    let rows = (body.height / 2).max(1) as usize;
    let selected = app.commit_selected.min(app.commits.len() - 1);
    let start = selected.saturating_sub(rows - 1);
    let end = (start + rows).min(app.commits.len());
    for (offset, index) in (start..end).enumerate() {
        let y = body.y + offset as u16 * 2;
        if y + 1 >= body.y + body.height {
            break;
        }
        let commit = &app.commits[index];
        let rect = Rect::new(body.x, y, body.width, 2);
        if index == selected {
            fill(frame, rect, theme.color("surface0"));
        }
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{} ", commit.short_sha()),
                    Style::default()
                        .fg(theme.color("peach"))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format::truncate(
                        &format::clean_text(commit.summary()),
                        body.width.saturating_sub(12) as usize,
                    ),
                    Style::default().fg(theme.color("text")),
                ),
            ])),
            Rect::new(body.x + 1, y, body.width.saturating_sub(1), 1),
        );
        let meta = format!(
            "   {} · {}",
            if commit.author_name().is_empty() {
                "未知"
            } else {
                commit.author_name()
            },
            format::relative_time(&commit.commit.author.date)
        );
        frame.render_widget(
            Paragraph::new(format::truncate(&meta, body.width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(body.x + 1, y + 1, body.width.saturating_sub(1), 1),
        );
        register(hits, rect, Action::SelectCommit(index));
    }
}

fn draw_commit_detail(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.color("surface0"))),
        area,
    );
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.width < 10 || inner.height < 4 {
        return;
    }
    let Some(commit) = app.selected_commit() else {
        draw_center_message(
            frame,
            theme,
            area,
            "挑一个提交看看喵~",
            "点左边列表，人家就把提交信息摆在这里喵~",
        );
        return;
    };
    frame.render_widget(
        Paragraph::new(format!("{} {}", icons::COMMIT, commit.short_sha())).style(
            Style::default()
                .fg(theme.color("peach"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let mut y = inner.y + 1;
    frame.render_widget(
        Paragraph::new(format!(
            "{} · {}",
            if commit.author_name().is_empty() {
                "未知"
            } else {
                commit.author_name()
            },
            format::relative_time(&commit.commit.author.date)
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 2;
    let button_y = inner.y + inner.height.saturating_sub(1);
    let room = button_y.saturating_sub(y) as usize;
    let detail = app
        .commit_detail
        .as_ref()
        .filter(|detail| detail.sha == commit.sha || detail.sha.is_empty());
    let loading = app
        .commit_detail_loading
        .as_deref()
        .is_some_and(|sha| sha == commit.sha);
    let reserve = usize::from(room > 1 && (detail.is_some() || loading));
    let message_budget = if detail.is_some() || loading {
        room.saturating_sub(reserve).min(4)
    } else {
        room
    };
    let message = format::clean_text(&commit.commit.message);
    for line in format::wrap_text(&message, inner.width as usize, message_budget) {
        frame.render_widget(
            Paragraph::new(line).style(Style::default().fg(theme.color("text"))),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
    if y < button_y {
        if let Some(detail) = detail {
            let additions = detail
                .stats
                .as_ref()
                .map(|stats| stats.additions)
                .unwrap_or_else(|| detail.files.iter().map(|file| file.additions).sum());
            let deletions = detail
                .stats
                .as_ref()
                .map(|stats| stats.deletions)
                .unwrap_or_else(|| detail.files.iter().map(|file| file.deletions).sum());
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        format!("{} 个文件 ", detail.files.len()),
                        Style::default().fg(theme.color("subtext0")),
                    ),
                    Span::styled(
                        format!("+{additions}"),
                        Style::default().fg(theme.color("green")),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("-{deletions}"),
                        Style::default().fg(theme.color("red")),
                    ),
                ])),
                Rect::new(inner.x, y, inner.width, 1),
            );
            y += 1;
            if detail.files.is_empty() && y < button_y {
                frame.render_widget(
                    Paragraph::new("没有文件改动喵~")
                        .style(Style::default().fg(theme.color("overlay0"))),
                    Rect::new(inner.x, y, inner.width, 1),
                );
                y += 1;
            }
            let rows = button_y.saturating_sub(y) as usize;
            let mut shown = 0;
            for file in detail.files.iter().take(rows) {
                let (marker, color) = match file.status.as_str() {
                    "added" => ("A", "green"),
                    "removed" => ("D", "red"),
                    "renamed" => ("R", "blue"),
                    _ => ("M", "peach"),
                };
                let diff = format!("+{} -{}", file.additions, file.deletions);
                let diff_width = format::text_width(&diff) as u16;
                let name_width = inner.width.saturating_sub(diff_width + 3);
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(
                            format!("{marker} "),
                            Style::default()
                                .fg(theme.color(color))
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format::truncate(
                                &format::clean_text(&file.filename),
                                name_width as usize,
                            ),
                            Style::default().fg(theme.color("subtext1")),
                        ),
                    ])),
                    Rect::new(inner.x, y, name_width, 1),
                );
                frame.render_widget(
                    Paragraph::new(diff)
                        .alignment(Alignment::Right)
                        .style(Style::default().fg(theme.color("overlay0"))),
                    Rect::new(inner.x, y, inner.width, 1),
                );
                y += 1;
                shown += 1;
            }
            if shown < detail.files.len() && y < button_y {
                frame.render_widget(
                    Paragraph::new(format!(
                        "还有 {} 个文件，o 去 GitHub 看全部喵~",
                        detail.files.len() - shown
                    ))
                    .style(Style::default().fg(theme.color("overlay0"))),
                    Rect::new(inner.x, y, inner.width, 1),
                );
            }
        } else if loading {
            frame.render_widget(
                Paragraph::new(format!("{} 正在读改动文件喵~...", app.spinner()))
                    .style(Style::default().fg(theme.color("peach"))),
                Rect::new(inner.x, y, inner.width, 1),
            );
        }
    }
    let open = icons::label(icons::EXTERNAL, "去 GitHub 看喵~");
    let open_width = format::text_width(&open) as u16 + 4;
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            open_width,
            1,
        ),
        &open,
        Action::OpenExternal(commit.html_url.clone()),
        true,
    );
}

fn draw_branches_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let width = area.width.saturating_sub(12).min(58);
    let height = area.height.saturating_sub(8).clamp(10, 24);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    app.layout.overlay = rect;
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
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
    frame.render_widget(
        Paragraph::new(icons::label(icons::BRANCH, "换个分支喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width.saturating_sub(10), 1),
    );
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x + inner.width.saturating_sub(9), inner.y, 9, 1),
        "关掉喵~",
        Action::CloseOverlay,
        false,
    );
    let current = app
        .page_repo
        .as_ref()
        .map(|repo| app.branch_for(repo))
        .unwrap_or_default();
    frame.render_widget(
        Paragraph::new(format!(
            "{} 个分支 · 当前 {}喵~",
            app.branches.len(),
            format::truncate(&current, 24)
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let list = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(4),
    );
    app.branch_viewport = list.height as usize;
    let max_offset = app
        .branches
        .len()
        .saturating_sub(app.branch_viewport.max(1));
    app.branch_offset = app.branch_offset.min(max_offset);
    if app.branches.is_empty() {
        draw_center_message(
            frame,
            theme,
            list,
            "分支还没到喵~",
            if app.branches_loading.is_some() {
                "人家正在取分支列表喵~..."
            } else {
                "按 r 再刷新一下嘛喵~"
            },
        );
    }
    for (row, branch) in app
        .branches
        .iter()
        .skip(app.branch_offset)
        .take(app.branch_viewport)
        .enumerate()
    {
        let index = app.branch_offset + row;
        let item = Rect::new(list.x, list.y + row as u16, list.width, 1);
        let selected = index == app.branch_selected;
        let active = branch.name == current;
        if selected || active {
            fill(
                frame,
                item,
                if selected {
                    theme.color("surface1")
                } else {
                    theme.color("surface0")
                },
            );
        }
        if active {
            fill(frame, Rect::new(item.x, item.y, 1, 1), theme.color("green"));
        }
        frame.render_widget(
            Paragraph::new(format!(
                "{} {}",
                if active { icons::CHECK } else { icons::BRANCH },
                format::truncate(&branch.name, item.width.saturating_sub(12) as usize)
            ))
            .style(if active {
                Style::default()
                    .fg(theme.color("green"))
                    .add_modifier(Modifier::BOLD)
            } else if selected {
                Style::default().fg(theme.color("text"))
            } else {
                Style::default().fg(theme.color("subtext1"))
            }),
            Rect::new(item.x + 1, item.y, item.width.saturating_sub(10), 1),
        );
        frame.render_widget(
            Paragraph::new(format::truncate(&branch.commit.sha, 7))
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(item.x, item.y, item.width.saturating_sub(2), 1),
        );
        register(hits, item, Action::SelectBranch(branch.name.clone()));
    }
    frame.render_widget(
        Paragraph::new("j/k 挑选   Enter 切换   r 刷新   Esc 关掉喵~")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            1,
        ),
    );
}

fn draw_actions_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    if inner.height < 8 {
        draw_center_message(
            frame,
            theme,
            inner,
            "这里有点挤喵~",
            "把终端拉大一点，人家才摆得下工作流页喵~",
        );
        return;
    }

    let back_label = icons::label(icons::BACK, "返回喵~");
    let back_width = format::text_width(&back_label) as u16 + 2;
    let back_rect = Rect::new(inner.x, inner.y, back_width, 1);
    let back_hover = app.hover.as_ref() == Some(&Action::Back);
    frame.render_widget(
        Paragraph::new(back_label).style(if back_hover {
            Style::default().fg(theme.color("blue"))
        } else {
            Style::default().fg(theme.color("subtext0"))
        }),
        back_rect,
    );
    register(hits, back_rect, Action::Back);

    let refresh_label = icons::label(icons::REFRESH, "刷新喵~");
    let refresh_width = format::text_width(&refresh_label) as u16 + 4;
    let refresh_x = inner.x + inner.width.saturating_sub(refresh_width);
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(refresh_x, inner.y, refresh_width, 1),
        &refresh_label,
        Action::Refresh,
        false,
    );

    let run_label = icons::label(icons::PLAY, "运行工作流喵~");
    let run_width = format::text_width(&run_label) as u16 + 4;
    let run_x = refresh_x.saturating_sub(run_width + 1);
    let title_x = inner.x + back_width + 2;
    let title_end = if run_x > title_x {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(run_x, inner.y, run_width, 1),
            &run_label,
            Action::OpenWorkflowRunner,
            true,
        );
        run_x
    } else {
        refresh_x
    };

    let repo = app
        .action_repo
        .clone()
        .unwrap_or_else(|| "还没有选仓库".to_string());
    let title = format!("{} 工作流喵~  {repo}", icons::COGS);
    let title_width = title_end.saturating_sub(title_x + 1) as usize;
    frame.render_widget(
        Paragraph::new(format::truncate(&title, title_width)).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(title_x, inner.y, title_width as u16, 1),
    );
    frame.render_widget(
        Paragraph::new("这里是当前仓库的 Actions 运行记录喵~")
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    fill(
        frame,
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
        theme.color("surface0"),
    );

    let content = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(3),
    );
    if content.height < 4 {
        return;
    }
    let columns = Layout::horizontal([
        Constraint::Percentage(44),
        Constraint::Length(1),
        Constraint::Min(24),
    ])
    .split(content);
    draw_action_runs_panel(frame, app, theme, columns[0], hits);
    draw_action_jobs_panel(frame, app, theme, columns[2], hits);
}

fn draw_action_runs_panel(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    frame.render_widget(
        Paragraph::new(icons::label(icons::COGS, "运行记录喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    frame.render_widget(
        Paragraph::new(format!("{} 条喵~", app.action_runs.len()))
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
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
    if app.action_runs.is_empty() {
        let (title, detail) = if app.action_runs_loading {
            ("人家正在翻工作流记录喵~...", "第一次同步要等一小会儿哦喵~")
        } else {
            (
                "还没有运行记录喵~",
                "这个仓库还没跑过 Actions，或者人家没权限看喵~",
            )
        };
        draw_center_message(frame, theme, body, title, detail);
        return;
    }

    let rows = (body.height / 2).max(1) as usize;
    let selected = app.action_selected.min(app.action_runs.len() - 1);
    let start = selected.saturating_sub(rows - 1);
    let end = (start + rows).min(app.action_runs.len());
    for (offset, index) in (start..end).enumerate() {
        let y = body.y + offset as u16 * 2;
        if y + 1 >= body.y + body.height {
            break;
        }
        let rect = Rect::new(body.x, y, body.width, 2);
        if index == selected {
            fill(frame, rect, theme.color("surface0"));
        }
        let run = &app.action_runs[index];
        let (icon, color) = workflow_status_icon(&run.status, run.conclusion.as_deref());
        let title_width = body.width.saturating_sub(5) as usize;
        let title = Line::from(vec![
            Span::styled(format!("{icon} "), Style::default().fg(theme.color(color))),
            Span::styled(
                format!("#{} ", run.run_number),
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format::truncate(&format::clean_text(run.title()), title_width),
                Style::default().fg(theme.color("text")),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(title),
            Rect::new(body.x + 1, y, body.width.saturating_sub(1), 1),
        );
        let event = if run.event.trim().is_empty() {
            "手动"
        } else {
            run.event.as_str()
        };
        let branch = if run.branch().is_empty() {
            "无分支"
        } else {
            run.branch()
        };
        let meta = format!(
            "   {} · {} · {} · {}",
            format::clean_text(event),
            format::clean_text(branch),
            run.status_label(),
            format::relative_time(&run.created_at)
        );
        frame.render_widget(
            Paragraph::new(format::truncate(&meta, body.width as usize))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(body.x + 1, y + 1, body.width.saturating_sub(1), 1),
        );
        register(hits, rect, Action::SelectActionRun(index));
    }
}

fn draw_action_jobs_panel(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    let Some(run) = app.selected_action_run() else {
        frame.render_widget(
            Paragraph::new(icons::label(icons::CODE, "任务与步骤喵~")).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
        );
        draw_center_message(
            frame,
            theme,
            Rect::new(
                area.x + 1,
                area.y + 2,
                area.width.saturating_sub(2),
                area.height.saturating_sub(2),
            ),
            "先在左边挑一条运行记录喵~",
            "挑好之后人家会把任务和步骤列出来喵~",
        );
        return;
    };
    frame.render_widget(
        Paragraph::new(icons::label(icons::CODE, "任务与步骤喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("#{} {}", run.run_number, format::clean_text(run.title())),
            area.width.saturating_sub(2) as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 1),
    );
    fill(
        frame,
        Rect::new(area.x, area.y + 2, area.width, 1),
        theme.color("surface0"),
    );
    let body = Rect::new(
        area.x + 1,
        area.y + 3,
        area.width.saturating_sub(2),
        area.height.saturating_sub(3),
    );
    if app.action_jobs.is_empty() {
        let (title, detail) = if app.action_jobs_loading == Some(run.id) {
            ("人家正在读任务详情喵~...", "任务多的时候要等一小会儿哦喵~")
        } else {
            (
                "还没有任务详情喵~",
                "这条运行可能还在排队，或者接口没给任务喵~",
            )
        };
        draw_center_message(frame, theme, body, title, detail);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let mut actions: Vec<(usize, Action)> = Vec::new();
    for job in &app.action_jobs {
        let (icon, color) = workflow_status_icon(&job.status, job.conclusion.as_deref());
        let duration = job
            .started_at
            .as_deref()
            .and_then(|start| format::duration_label(start, job.completed_at.as_deref()))
            .unwrap_or_else(|| "未开始".to_string());
        let name = format::truncate(
            &format::clean_text(&job.name),
            body.width.saturating_sub(20) as usize,
        );
        lines.push(Line::from(vec![
            Span::styled(format!("{icon} "), Style::default().fg(theme.color(color))),
            Span::styled(name, Style::default().fg(theme.color("text"))),
            Span::styled(
                format!("  {}  {}", job.status_label(), duration),
                Style::default().fg(theme.color("overlay0")),
            ),
        ]));
        if !job.html_url.is_empty() {
            actions.push((lines.len() - 1, Action::OpenExternal(job.html_url.clone())));
        }
        for step in &job.steps {
            let (step_icon, step_color) =
                workflow_status_icon(&step.status, step.conclusion.as_deref());
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {step_icon} "),
                    Style::default().fg(theme.color(step_color)),
                ),
                Span::styled(
                    format::truncate(
                        &format::clean_text(&step.name),
                        body.width.saturating_sub(16) as usize,
                    ),
                    Style::default().fg(theme.color("subtext0")),
                ),
                Span::styled(
                    format!("  {}", step.status_label()),
                    Style::default().fg(theme.color("overlay0")),
                ),
            ]));
        }
        lines.push(Line::default());
    }
    lines.push(Line::from(Span::styled(
        "点任务行或按 Enter 去浏览器看完整日志喵~",
        Style::default().fg(theme.color("overlay0")),
    )));
    app.action_jobs_scroll_max = lines.len().saturating_sub(body.height as usize) as u16;
    app.action_jobs_scroll = app.action_jobs_scroll.min(app.action_jobs_scroll_max);
    render_scrolled(
        frame,
        body,
        lines,
        actions,
        app.action_jobs_scroll,
        theme,
        hits,
    );
}

fn draw_cache_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let groups = cache_groups();
    let width = area.width.saturating_sub(12).min(72);
    let height = (groups.len() as u16 + 7).min(area.height.saturating_sub(4));
    if width < 30 || height < 6 {
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
            .border_style(Style::default().fg(theme.color("blue"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::DATABASE, "缓存管理喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!(
            "共 {} 个文件 · {}喵~",
            app.cache_stats.files,
            format::format_size(app.cache_stats.bytes)
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let list_top = inner.y + 3;
    let bottom = inner.y + inner.height.saturating_sub(1);
    for (index, group) in groups.iter().enumerate() {
        let y = list_top + index as u16;
        if y >= bottom {
            break;
        }
        let stats = app.cache_group_stats(index);
        let selected = index == app.cache_row;
        let row = Rect::new(inner.x, y, inner.width, 1);
        if selected {
            fill(frame, row, theme.color("surface0"));
        }
        let label = format!(
            "{} {:<10}  {:>3} 个文件 · {}",
            icons::DATABASE,
            group.label,
            stats.files,
            format::format_size(stats.bytes)
        );
        frame.render_widget(
            Paragraph::new(format::truncate(&label, inner.width as usize)).style(if selected {
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.color("subtext0"))
            }),
            row,
        );
        register(hits, row, Action::SelectCacheRow(index));
    }

    let mut x = inner.x;
    let clear_label = icons::label(icons::TRASH, "清掉选中喵~");
    let clear_width = format::text_width(&clear_label) as u16 + 4;
    if x + clear_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, clear_width, 1),
            &clear_label,
            Action::ClearCacheGroup(app.cache_row),
            false,
        );
        x += clear_width + 1;
    }
    let open_label = icons::label(icons::FOLDER_OPEN, "缓存目录喵~");
    let open_width = format::text_width(&open_label) as u16 + 4;
    if x + open_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, open_width, 1),
            &open_label,
            Action::OpenCacheDir,
            false,
        );
        x += open_width + 1;
    }
    let close_label = "关掉喵~";
    let close_width = format::text_width(close_label) as u16 + 4;
    if x + close_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, close_width, 1),
            close_label,
            Action::CloseOverlay,
            false,
        );
    }
    app.layout.overlay = rect;
}

#[allow(clippy::too_many_arguments)]
fn draw_workflow_field(
    frame: &mut Frame,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    x: u16,
    y: u16,
    width: u16,
    label: &str,
    input: &TextInput,
    editing: bool,
    placeholder: &str,
    action: Action,
) {
    let label_width = format::text_width(label) as u16;
    frame.render_widget(
        Paragraph::new(format::truncate(label, width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(x, y, width, 1),
    );
    let input_x = x + label_width.saturating_add(2);
    let input_width = width.saturating_sub(label_width.saturating_add(2));
    let input_rect = Rect::new(input_x, y, input_width, 1);
    if input_width > 0 {
        fill(
            frame,
            input_rect,
            if editing {
                theme.color("surface1")
            } else {
                theme.color("surface0")
            },
        );
        let empty = input.is_empty();
        let shown = if empty {
            placeholder.to_string()
        } else {
            input.value.clone()
        };
        frame.render_widget(
            Paragraph::new(format::truncate(&shown, input_width as usize)).style(if empty {
                Style::default().fg(theme.color("overlay0"))
            } else {
                Style::default().fg(theme.color("text"))
            }),
            input_rect,
        );
        if editing {
            let cursor_x = if empty {
                input_x
            } else {
                input_x + (input.cursor as u16).min(input_width.saturating_sub(1))
            };
            frame.set_cursor_position(Position::new(cursor_x, y));
        }
    }
    register(hits, Rect::new(x, y, width, 1), action);
}

fn draw_workflow_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let width = area.width.saturating_sub(10).min(80);
    let height = area.height.saturating_sub(6).min(26);
    if width < 40 || height < 14 {
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
            .border_style(Style::default().fg(theme.color("blue"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::PLAY, "运行工作流喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let repo = app
        .action_repo
        .clone()
        .unwrap_or_else(|| "还没有选仓库".to_string());
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("{repo} · 只有支持 workflow_dispatch 的工作流才能手动运行喵~"),
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let list_top = inner.y + 3;
    let list_height = inner.height.saturating_sub(11).max(2);
    let list_area = Rect::new(inner.x, list_top, inner.width, list_height);
    if app.action_workflows.is_empty() {
        let (title, detail) = if app.action_workflows_loading {
            (
                "人家正在取工作流列表喵~...",
                "仓库里的 YAML 有点多的话要等一小会儿喵~",
            )
        } else {
            (
                "这个仓库还没有工作流喵~",
                "先往 .github/workflows 里放一个 workflow_dispatch 吧喵~",
            )
        };
        draw_center_message(frame, theme, list_area, title, detail);
    } else {
        let rows = list_height as usize;
        let selected = app.workflow_selected.min(app.action_workflows.len() - 1);
        let start = selected.saturating_sub(rows.saturating_sub(1));
        let end = (start + rows).min(app.action_workflows.len());
        for (offset, index) in (start..end).enumerate() {
            let y = list_area.y + offset as u16;
            let row = Rect::new(list_area.x, y, list_area.width, 1);
            let workflow = &app.action_workflows[index];
            if index == selected {
                fill(frame, row, theme.color("surface0"));
            }
            let name = if workflow.name.trim().is_empty() {
                workflow.file_name()
            } else {
                workflow.name.as_str()
            };
            let label = format!(
                "{} {}  {}  {}",
                icons::COGS,
                format::clean_text(name),
                workflow.state_label(),
                format::clean_text(&workflow.path)
            );
            let style = if index == selected {
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.color("subtext0"))
            };
            frame.render_widget(
                Paragraph::new(format::truncate(&label, list_area.width as usize)).style(style),
                row,
            );
            register(hits, row, Action::SelectWorkflow(index));
        }
    }

    let ref_y = list_top + list_height + 1;
    draw_workflow_field(
        frame,
        theme,
        hits,
        inner.x,
        ref_y,
        inner.width,
        "分支/标签喵~",
        &app.workflow_ref,
        app.workflow_ref_editing,
        "默认分支喵~",
        Action::EditWorkflowRef,
    );
    draw_workflow_field(
        frame,
        theme,
        hits,
        inner.x,
        ref_y + 1,
        inner.width,
        "inputs JSON喵~",
        &app.workflow_inputs,
        app.workflow_inputs_editing,
        "{}",
        Action::EditWorkflowInputs,
    );

    if let Some(error) = app.workflow_error.as_deref() {
        frame.render_widget(
            Paragraph::new(format::truncate(error, inner.width as usize))
                .style(Style::default().fg(theme.color("red"))),
            Rect::new(inner.x, ref_y + 2, inner.width, 1),
        );
    }
    frame.render_widget(
        Paragraph::new(format::truncate(
            "j/k 挑  Tab 换输入框  Enter 运行  r 刷新列表  o 开工作流  Esc 关掉喵~",
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, ref_y + 3, inner.width, 1),
    );

    let bottom = inner.y + inner.height.saturating_sub(1);
    let mut x = inner.x;
    let run_label = if app.workflow_running {
        format!("{} 提交中喵~", app.spinner())
    } else {
        icons::label(icons::PLAY, "运行喵~")
    };
    let run_width = format::text_width(&run_label) as u16 + 4;
    if x + run_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, run_width, 1),
            &run_label,
            Action::RunWorkflow,
            true,
        );
        x += run_width + 1;
    }
    let refresh_label = icons::label(icons::REFRESH, "刷新列表喵~");
    let refresh_width = format::text_width(&refresh_label) as u16 + 4;
    if x + refresh_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, refresh_width, 1),
            &refresh_label,
            Action::RefreshWorkflows,
            false,
        );
        x += refresh_width + 1;
    }
    let close_label = "关掉喵~";
    let close_width = format::text_width(close_label) as u16 + 4;
    if x + close_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, close_width, 1),
            close_label,
            Action::CloseOverlay,
            false,
        );
    }
    app.layout.overlay = rect;
}

fn draw_fork_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let width = area.width.saturating_sub(12).min(68);
    let height = area.height.saturating_sub(8).min(15);
    if width < 38 || height < 10 {
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
            .border_style(Style::default().fg(theme.color("blue"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::FORK, "Fork 仓库喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let Some(repo) = app.fork_repo.as_ref() else {
        draw_center_message(
            frame,
            theme,
            inner,
            "没有选仓库喵~",
            "关掉这个小窗口再挑一个嘛喵~",
        );
        app.layout.overlay = rect;
        return;
    };
    let mut y = inner.y + 2;
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("{} {}", icons::REPO, repo.display_full_name()),
            inner.width as usize,
        ))
        .style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 1;
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!(
                "作者 @{}   {} 星标   {} Fork   默认分支 {}喵~",
                repo.owner_name(),
                format::human_count(repo.stargazers_count),
                format::human_count(repo.forks_count),
                app.branch_for(repo)
            ),
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 2;
    let target = app.login.as_deref().unwrap_or("未登录");
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("{} 会复制到你的账号 @{} 下面喵~", icons::USER, target),
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("blue"))),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 1;
    frame.render_widget(
        Paragraph::new(format::truncate(
            "Fork 只是复制一份，原仓库不会被改动喵~",
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 1;
    frame.render_widget(
        Paragraph::new(format::truncate(
            "之后可以在自己的账号里继续改，或者再发合并请求喵~",
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, y, inner.width, 1),
    );

    let bottom = inner.y + inner.height.saturating_sub(1);
    let mut x = inner.x;
    let confirm = if app.fork_loading.is_some() {
        format!("{} 正在 Fork 喵~", app.spinner())
    } else {
        icons::label(icons::FORK, "开始 Fork 喵~")
    };
    let confirm_width = format::text_width(&confirm) as u16 + 4;
    if x + confirm_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, confirm_width, 1),
            &confirm,
            Action::ConfirmFork,
            true,
        );
        x += confirm_width + 1;
    }
    let cancel = "再想想喵~";
    let cancel_width = format::text_width(cancel) as u16 + 4;
    if x + cancel_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, cancel_width, 1),
            cancel,
            Action::CloseOverlay,
            false,
        );
    }
    app.layout.overlay = rect;
}

fn draw_delete_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let width = area.width.saturating_sub(14).min(60);
    let height = area.height.saturating_sub(10).min(12);
    if width < 34 || height < 9 {
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
            .border_style(Style::default().fg(theme.color("red"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::TRASH, "删除仓库喵~")).style(
            Style::default()
                .fg(theme.color("red"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let Some(repo) = app.delete_repo.as_ref() else {
        draw_center_message(
            frame,
            theme,
            inner,
            "没有选仓库喵~",
            "关掉这个小窗口再挑一个嘛喵~",
        );
        app.layout.overlay = rect;
        return;
    };
    let mut y = inner.y + 2;
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("{} {}", icons::REPO, repo.display_full_name()),
            inner.width as usize,
        ))
        .style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 1;
    frame.render_widget(
        Paragraph::new(format::truncate(
            "这个操作不能撤销喵！仓库里的议题、发行版和 Actions 记录都会一起消失喵~",
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("peach"))),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 1;
    frame.render_widget(
        Paragraph::new(format::truncate(
            "确定要从 GitHub 删除这个仓库吗喵？",
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("red"))),
        Rect::new(inner.x, y, inner.width, 1),
    );

    let bottom = inner.y + inner.height.saturating_sub(1);
    let mut x = inner.x;
    let confirm = if app.delete_loading.is_some() {
        format!("{} 正在删除喵~", app.spinner())
    } else {
        icons::label(icons::TRASH, "确认删除喵~")
    };
    let confirm_width = format::text_width(&confirm) as u16 + 4;
    if x + confirm_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, confirm_width, 1),
            &confirm,
            Action::ConfirmDeleteRepo,
            true,
        );
        x += confirm_width + 1;
    }
    let cancel = "取消喵~";
    let cancel_width = format::text_width(cancel) as u16 + 4;
    if x + cancel_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, bottom, cancel_width, 1),
            cancel,
            Action::CloseOverlay,
            false,
        );
    }
    app.layout.overlay = rect;
}

fn draw_file_tree_panel(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    repo: &Repo,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    register(hits, area, Action::FocusRepoPane(RepoPane::Tree));
    let focused = app.repo_pane == RepoPane::Tree;
    let title = icons::label(icons::FOLDER_OPEN, "文件树喵~");
    let counts = app
        .tree
        .as_ref()
        .map(|tree| {
            let directories = tree
                .tree
                .iter()
                .filter(|entry| entry.kind == "tree")
                .count();
            let files = tree
                .tree
                .iter()
                .filter(|entry| entry.kind == "blob")
                .count();
            format!("{directories} 目录 · {files} 文件")
        })
        .unwrap_or_default();
    let heading = format!(
        "{}   {} {}   {counts}",
        title,
        icons::BRANCH,
        format::truncate(&app.branch_for(repo), 18)
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &heading,
            area.width.saturating_sub(2) as usize,
        ))
        .style(if focused {
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
    let body = Rect::new(
        area.x + 1,
        area.y + 2,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    if !app.config.show_file_tree {
        draw_center_message(
            frame,
            theme,
            body,
            "文件树预览没开喵~",
            "到设置页里手动打开，人家才会去取文件树哦喵~",
        );
        app.tree_scroll = 0;
        app.tree_scroll_max = 0;
        return;
    }
    if let Some(loading) = app.tree_loading.as_ref()
        && loading == &repo.display_full_name()
    {
        frame.render_widget(
            Paragraph::new(format!("{} 人家正在翻文件树喵~...", app.spinner()))
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(body.x, body.y, body.width, 1),
        );
        return;
    }
    let Some(tree) = app.tree.as_ref() else {
        draw_center_message(
            frame,
            theme,
            body,
            "文件树还没到喵~",
            "点一下刷新，或者稍后再看看嘛喵~",
        );
        return;
    };
    let mut entries: Vec<_> = tree
        .tree
        .iter()
        .filter(|entry| entry.kind == "blob" || entry.kind == "tree")
        .collect();
    entries.sort_by_cached_key(|entry| tree_sort_key(&entry.path, entry.kind == "tree"));
    if entries.is_empty() {
        draw_center_message(
            frame,
            theme,
            body,
            "仓库里空空的喵~",
            "这个分支下面还没有文件呢喵~",
        );
        return;
    }
    let mut last_child: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for entry in &entries {
        let (parent, name) = path_parent_and_name(&entry.path);
        last_child.insert(parent, name);
    }
    let mut lines: Vec<Line> = Vec::new();
    let mut actions = Vec::new();
    for entry in entries.iter().take(1200) {
        let (parent, name) = path_parent_and_name(&entry.path);
        let directory = entry.kind == "tree";
        let icon = if directory {
            icons::FOLDER_OPEN
        } else {
            file_icon(&name)
        };
        let parts: Vec<&str> = entry.path.split('/').collect();
        let depth = parts.len().saturating_sub(1).min(8);
        let mut prefix = String::new();
        for level in 0..depth {
            let ancestor = parts[..=level].join("/");
            let (ancestor_parent, ancestor_name) = path_parent_and_name(&ancestor);
            let ancestor_is_last = last_child
                .get(&ancestor_parent)
                .is_some_and(|last| last == &ancestor_name);
            prefix.push_str(if ancestor_is_last { "   " } else { "│  " });
        }
        let is_last = last_child.get(&parent).is_some_and(|last| last == &name);
        prefix.push_str(if is_last { "└─ " } else { "├─ " });
        let prefix_width = format::text_width(&prefix);
        let size_text = if directory {
            String::new()
        } else {
            entry.size.map(format::format_size).unwrap_or_default()
        };
        let size_width = format::text_width(&size_text);
        let label = format::truncate(
            &format::clean_text(&name),
            (body.width as usize)
                .saturating_sub(prefix_width + size_width + 3)
                .max(1),
        );
        let action = Action::OpenTreePath {
            path: entry.path.clone(),
            directory,
        };
        actions.push((lines.len(), action));
        let mut spans = vec![
            Span::styled(prefix, Style::default().fg(theme.color("surface2"))),
            Span::styled(
                format!("{icon} "),
                Style::default().fg(if directory {
                    theme.color("blue")
                } else {
                    theme.color("overlay1")
                }),
            ),
            Span::styled(
                label.clone(),
                Style::default().fg(if directory {
                    theme.color("subtext1")
                } else {
                    theme.color("subtext0")
                }),
            ),
        ];
        if size_width > 0 && body.width as usize > prefix_width + size_width + 10 {
            let pad = (body.width as usize)
                .saturating_sub(prefix_width + size_width + 3 + format::text_width(&label));
            spans.push(Span::styled(" ".repeat(pad), Style::default()));
            spans.push(Span::styled(
                size_text,
                Style::default().fg(theme.color("overlay0")),
            ));
        }
        lines.push(Line::from(spans));
    }
    if tree.truncated || entries.len() > 1200 {
        lines.push(Line::from(Span::styled(
            "文件太多啦，只展示了一部分喵~",
            Style::default().fg(theme.color("peach")),
        )));
    }
    app.tree_scroll_max = lines.len().saturating_sub(body.height as usize) as u16;
    app.tree_scroll = app.tree_scroll.min(app.tree_scroll_max);
    render_scrolled(frame, body, lines, actions, app.tree_scroll, theme, hits);
}

fn tree_sort_key(path: &str, directory: bool) -> Vec<(u8, String)> {
    let mut parts: Vec<&str> = path.split('/').collect();
    let last = parts.pop().unwrap_or_default();
    let mut key: Vec<(u8, String)> = parts.iter().map(|part| (0, part.to_lowercase())).collect();
    key.push((u8::from(!directory), last.to_lowercase()));
    key
}

fn path_parent_and_name(path: &str) -> (String, String) {
    match path.rsplit_once('/') {
        Some((parent, name)) => (parent.to_string(), name.to_string()),
        None => (String::new(), path.to_string()),
    }
}

fn draw_releases_panel(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    repo: &Repo,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    register(hits, area, Action::FocusRepoPane(RepoPane::Releases));
    let focused = app.repo_pane == RepoPane::Releases;
    frame.render_widget(
        Paragraph::new(icons::label(icons::TAG, "发行版喵~")).style(if focused {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("text"))
        }),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    let dir_label = format!("{} {}", icons::DOWNLOAD, app.download_path_label());
    frame.render_widget(
        Paragraph::new(format::truncate(
            &dir_label,
            area.width.saturating_sub(2) as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 1),
    );
    draw_filter_line(
        frame,
        theme,
        Rect::new(area.x + 1, area.y + 2, area.width.saturating_sub(2), 1),
        hits,
        &app.release_filter,
        app.release_filter_focus,
        app.hover.as_ref() == Some(&Action::ReleaseFilterFocus),
        Action::ReleaseFilterFocus,
        "按 / 搜发行版喵~",
    );
    fill(
        frame,
        Rect::new(area.x, area.y + 3, area.width, 1),
        theme.color("surface0"),
    );
    let body = Rect::new(
        area.x + 1,
        area.y + 4,
        area.width.saturating_sub(2),
        area.height.saturating_sub(5),
    );
    if let Some(loading) = app.releases_loading.as_ref()
        && loading == &repo.display_full_name()
    {
        frame.render_widget(
            Paragraph::new(format!("{} 人家正在取发行版喵~...", app.spinner()))
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(body.x, body.y, body.width, 1),
        );
        return;
    }
    if app.releases.is_empty() {
        draw_center_message(
            frame,
            theme,
            body,
            "还没有发行版喵~",
            "等作者发一个，或者按 r 再刷新一下嘛喵~",
        );
        return;
    }
    let indices = app.visible_release_indices();
    if indices.is_empty() {
        draw_center_message(
            frame,
            theme,
            body,
            "没找到发行版喵~",
            "按 / 改改关键词，或者按 Esc 离开筛选喵~",
        );
        return;
    }
    let rows = body.height as usize;
    app.page_scroll_max = indices.len().saturating_sub(rows) as u16;
    app.page_scroll = app.page_scroll.min(app.page_scroll_max);
    let start = app.page_scroll as usize;
    let end = (start + rows).min(indices.len());
    for (offset, position) in (start..end).enumerate() {
        let index = indices[position];
        let release = &app.releases[index];
        let y = body.y + offset as u16;
        let rect = Rect::new(body.x, y, body.width, 1);
        let action = Action::OpenReleaseDetail(index);
        let hovered = app.hover.as_ref() == Some(&action);
        if hovered {
            fill(frame, rect, theme.color("surface0"));
        } else {
            fill(frame, rect, theme.color("mantle"));
        }
        let meta = if body.width >= 36 {
            format!(
                "{} 个文件 · {} ",
                release.assets.len(),
                format::relative_time(release.date())
            )
        } else {
            format!("{} 个文件 ", release.assets.len())
        };
        let meta_width = format::text_width(&meta) as u16;
        let badge_width = if index == 0 {
            9
        } else if release.prerelease {
            11
        } else {
            0
        };
        let tag_max = rect
            .width
            .saturating_sub(meta_width + badge_width + 6)
            .max(4) as usize;
        let tag_text = format::truncate(&release.tag_name, tag_max);
        let accent = if release.draft {
            theme.color("red")
        } else if release.prerelease {
            theme.color("mauve")
        } else {
            theme.color("blue")
        };
        let mut title = vec![
            Span::styled(
                format!("{} ", icons::TAG),
                Style::default().fg(theme.color("peach")),
            ),
            release_tag_pill(theme, &tag_text, accent),
        ];
        if index == 0 {
            title.push(release_badge(icons::SPARKLE, "最新", theme.color("green")));
        } else if release.prerelease {
            title.push(release_badge(icons::CLOCK, "预发布", theme.color("peach")));
        }
        frame.render_widget(
            Paragraph::new(Line::from(title)),
            Rect::new(rect.x + 1, rect.y, rect.width.saturating_sub(1), 1),
        );
        frame.render_widget(
            Paragraph::new(format::truncate(
                &meta,
                body.width.saturating_sub(2) as usize,
            ))
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(rect.x, rect.y, rect.width.saturating_sub(1), 1),
        );
        register(hits, rect, action);
    }
}

fn draw_readme_panel(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    fill(frame, area, theme.color("mantle"));
    register(hits, area, Action::FocusRepoPane(RepoPane::Readme));
    let focused = app.repo_pane == RepoPane::Readme;
    let repo_name = app
        .page_repo
        .as_ref()
        .map(|repo| repo.display_full_name())
        .unwrap_or_default();
    let title_text = if app.readme_fullscreen {
        icons::label(icons::BOOK, "README · 全屏喵~")
    } else {
        icons::label(icons::BOOK, "README")
    };
    frame.render_widget(
        Paragraph::new(title_text).style(if focused || app.readme_fullscreen {
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("text"))
        }),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    if let Some(readme) = app.readme.as_ref() {
        let open_label = icons::label(icons::EXTERNAL, "去 GitHub 看喵~");
        let open_width = format::text_width(&open_label) as u16 + 2;
        let open_rect = Rect::new(
            area.x + area.width.saturating_sub(open_width),
            area.y,
            open_width,
            1,
        );
        let hovered = app.hover == Some(Action::OpenExternal(readme.html_url.clone()));
        frame.render_widget(
            Paragraph::new(open_label).style(if hovered {
                Style::default().fg(theme.color("blue"))
            } else {
                Style::default().fg(theme.color("overlay0"))
            }),
            open_rect,
        );
        register(
            hits,
            open_rect,
            Action::OpenExternal(readme.html_url.clone()),
        );
        if app.readme_fullscreen {
            let exit_label = icons::label(icons::BACK, "退出全屏喵~");
            let exit_width = format::text_width(&exit_label) as u16 + 4;
            let exit_x = open_rect.x.saturating_sub(exit_width + 1);
            if exit_x > area.x + 12 {
                draw_button(
                    frame,
                    app,
                    theme,
                    hits,
                    Rect::new(exit_x, area.y, exit_width, 1),
                    &exit_label,
                    Action::ToggleReadmeFullscreen,
                    true,
                );
            }
        } else {
            let full_label = icons::label(icons::EXPAND, "全屏喵~");
            let full_width = format::text_width(&full_label) as u16 + 4;
            let full_x = open_rect.x.saturating_sub(full_width + 1);
            if full_x > area.x + 12 {
                draw_button(
                    frame,
                    app,
                    theme,
                    hits,
                    Rect::new(full_x, area.y, full_width, 1),
                    &full_label,
                    Action::ToggleReadmeFullscreen,
                    false,
                );
            }
            let path_width = full_x.saturating_sub(area.x + 13);
            if path_width > 4 {
                frame.render_widget(
                    Paragraph::new(format::truncate(
                        &format::clean_text(&readme.path),
                        path_width as usize,
                    ))
                    .style(Style::default().fg(theme.color("overlay0"))),
                    Rect::new(area.x + 12, area.y, path_width, 1),
                );
            }
        }
    }
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
    if let Some(loading) = app.readme_loading.as_ref()
        && loading == &repo_name
    {
        frame.render_widget(
            Paragraph::new(format!("{} 人家正在读 README 喵~...", app.spinner()))
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(body.x, body.y, body.width, 1),
        );
        return;
    }
    let Some(readme) = app.readme.as_ref() else {
        draw_center_message(
            frame,
            theme,
            body,
            "README 还没到喵~",
            "点一下刷新，或者稍后再看看嘛喵~",
        );
        return;
    };
    let lines = markdown_lines(&readme.content, body.width as usize, theme);
    app.readme_scroll_max = lines.len().saturating_sub(body.height as usize) as u16;
    app.readme_scroll = app.readme_scroll.min(app.readme_scroll_max);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((app.readme_scroll, 0)),
        body,
    );
}

fn file_icon(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".md") || lower.ends_with(".markdown") {
        icons::BOOK
    } else if lower.ends_with(".rs")
        || lower.ends_with(".py")
        || lower.ends_with(".js")
        || lower.ends_with(".ts")
        || lower.ends_with(".go")
        || lower.ends_with(".toml")
        || lower.ends_with(".json")
        || lower.ends_with(".yaml")
        || lower.ends_with(".yml")
    {
        icons::CODE_FILE
    } else {
        icons::FILE
    }
}

fn inline_segments(text: &str, base: Style, theme: &Theme) -> Vec<(String, Style)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<(String, Style)> = Vec::new();
    let mut plain = String::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch == '\\' && index + 1 < chars.len() {
            plain.push(chars[index + 1]);
            index += 2;
            continue;
        }
        if ch == '!'
            && chars.get(index + 1) == Some(&'[')
            && let Some((alt, url, next)) = parse_link(&chars, index + 1)
        {
            flush_plain(&mut out, &mut plain, base);
            push_image_chip(&mut out, &alt, &url, base, theme);
            index = next;
            continue;
        }
        if ch == '['
            && let Some((label, url, next)) = parse_link(&chars, index)
        {
            flush_plain(&mut out, &mut plain, base);
            let style = base
                .fg(theme.color("blue"))
                .add_modifier(Modifier::UNDERLINED);
            let inner = inline_segments(&label, style, theme);
            let has_text = inner.iter().any(|(text, _)| !text.trim().is_empty());
            out.extend(inner);
            if has_text && (url.starts_with("http://") || url.starts_with("https://")) {
                out.push((" ↗".to_string(), base.fg(theme.color("blue"))));
            }
            index = next;
            continue;
        }
        if ch == '`'
            && let Some(end) = chars[index + 1..].iter().position(|item| *item == '`')
        {
            let end = index + 1 + end;
            let code: String = chars[index + 1..end].iter().collect();
            flush_plain(&mut out, &mut plain, base);
            if !code.is_empty() {
                out.push((
                    format!(" {code} "),
                    base.fg(theme.color("green")).bg(theme.color("surface0")),
                ));
            }
            index = end + 1;
            continue;
        }
        if ch == '<'
            && let Some(end) = chars[index + 1..].iter().position(|item| *item == '>')
        {
            let end = index + 1 + end;
            let inner: String = chars[index + 1..end].iter().collect();
            flush_plain(&mut out, &mut plain, base);
            if inner.starts_with("http://")
                || inner.starts_with("https://")
                || inner.starts_with("mailto:")
            {
                out.push((
                    inner,
                    base.fg(theme.color("blue"))
                        .add_modifier(Modifier::UNDERLINED),
                ));
            } else if let Some(src) = html_image_source(&inner) {
                push_image_chip(&mut out, "", &src, base, theme);
            }
            index = end + 1;
            continue;
        }
        if ch == '*' || ch == '_' {
            let double = chars.get(index + 1) == Some(&ch);
            let marker_len = if double { 2 } else { 1 };
            let after_word = index > 0 && chars[index - 1].is_alphanumeric();
            if !(ch == '_' && after_word)
                && let Some(end) = find_marker(&chars, index + marker_len, marker_len, ch)
                && end > index + marker_len
            {
                let inner: String = chars[index + marker_len..end].iter().collect();
                flush_plain(&mut out, &mut plain, base);
                let style = base.add_modifier(if double {
                    Modifier::BOLD
                } else {
                    Modifier::ITALIC
                });
                out.extend(inline_segments(&inner, style, theme));
                index = end + marker_len;
                continue;
            }
        }
        if ch == '~'
            && chars.get(index + 1) == Some(&'~')
            && let Some(end) = find_marker(&chars, index + 2, 2, '~')
            && end > index + 2
        {
            let inner: String = chars[index + 2..end].iter().collect();
            flush_plain(&mut out, &mut plain, base);
            out.extend(inline_segments(
                &inner,
                base.add_modifier(Modifier::CROSSED_OUT),
                theme,
            ));
            index = end + 2;
            continue;
        }
        if ch == 'h'
            && (starts_with(&chars, index, "https://") || starts_with(&chars, index, "http://"))
        {
            let mut end = index;
            while end < chars.len() && !chars[end].is_whitespace() {
                end += 1;
            }
            flush_plain(&mut out, &mut plain, base);
            let url: String = chars[index..end].iter().collect();
            out.push((
                url,
                base.fg(theme.color("blue"))
                    .add_modifier(Modifier::UNDERLINED),
            ));
            index = end;
            continue;
        }
        plain.push(if ch == '\t' { ' ' } else { ch });
        index += 1;
    }
    flush_plain(&mut out, &mut plain, base);
    out
}

fn push_image_chip(
    out: &mut Vec<(String, Style)>,
    alt: &str,
    url: &str,
    base: Style,
    theme: &Theme,
) {
    if is_badge_url(url) {
        return;
    }
    let label = alt.trim();
    let text = if label.is_empty() {
        if url.starts_with("http") {
            "图片".to_string()
        } else {
            format!("图片 {url}")
        }
    } else {
        format!("图片 {label}")
    };
    out.push((
        format!(" {} {text} ", icons::IMAGE),
        base.fg(theme.color("mauve")).bg(theme.color("surface0")),
    ));
}

fn is_badge_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    [
        "img.shields.io",
        "shields.io",
        "badgen.net",
        "badge",
        "travis-ci",
        "codecov",
        "coveralls",
        "cirrus-ci",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn html_image_source(tag: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    if !lower.trim_start().starts_with("img") {
        return None;
    }
    let start = lower.find("src")?;
    let rest = tag.get(start + 3..)?;
    let equal = rest.find('=')?;
    let value = rest.get(equal + 1..)?.trim_start();
    let quote = value.chars().next()?;
    if quote == '"' || quote == '\'' {
        let end = value.get(1..)?.find(quote)?;
        Some(value.get(1..1 + end)?.to_string())
    } else {
        Some(
            value
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .trim_end_matches('>')
                .to_string(),
        )
    }
}

fn flush_plain(out: &mut Vec<(String, Style)>, plain: &mut String, style: Style) {
    if !plain.is_empty() {
        out.push((std::mem::take(plain), style));
    }
}

fn starts_with(chars: &[char], index: usize, needle: &str) -> bool {
    needle
        .chars()
        .enumerate()
        .all(|(offset, ch)| chars.get(index + offset) == Some(&ch))
}

fn find_marker(chars: &[char], from: usize, len: usize, mark: char) -> Option<usize> {
    let mut index = from;
    while index + len <= chars.len() {
        if chars[index..index + len].iter().all(|ch| *ch == mark) {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn parse_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    if chars.get(start) != Some(&'[') {
        return None;
    }
    let mut label = String::new();
    let mut depth = 0usize;
    let mut index = start;
    while index < chars.len() {
        match chars[index] {
            '[' => {
                depth += 1;
                if depth > 1 {
                    label.push('[');
                }
            }
            ']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                label.push(']');
            }
            _ => label.push(chars[index]),
        }
        index += 1;
    }
    if depth != 0 || chars.get(index + 1) != Some(&'(') {
        return None;
    }
    let mut url = String::new();
    let mut paren = 1usize;
    let mut cursor = index + 2;
    while cursor < chars.len() {
        match chars[cursor] {
            '(' => {
                paren += 1;
                url.push('(');
            }
            ')' => {
                paren -= 1;
                if paren == 0 {
                    break;
                }
                url.push(')');
            }
            _ => url.push(chars[cursor]),
        }
        cursor += 1;
    }
    if paren != 0 {
        return None;
    }
    Some((label, url, cursor + 1))
}

fn color_mix(start: Color, end: Color, t: f32) -> Color {
    let (Color::Rgb(sr, sg, sb), Color::Rgb(er, eg, eb)) = (start, end) else {
        return start;
    };
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Color::Rgb(mix(sr, er), mix(sg, eg), mix(sb, eb))
}

fn gradient_chars(count: usize, glyph: &str, start: Color, end: Color) -> Vec<Span<'static>> {
    if count == 0 {
        return Vec::new();
    }
    let denominator = count.saturating_sub(1).max(1) as f32;
    (0..count)
        .map(|index| {
            let t = index as f32 / denominator;
            Span::styled(
                glyph.to_string(),
                Style::default().fg(color_mix(start, end, t)),
            )
        })
        .collect()
}

fn gradient_line(count: usize, glyph: &str, start: Color, end: Color) -> Line<'static> {
    Line::from(gradient_chars(count.max(1), glyph, start, end))
}

fn gradient_segments(
    segments: &[(String, Style)],
    start: Color,
    end: Color,
) -> Vec<(String, Style)> {
    let total = segments
        .iter()
        .map(|(text, _)| text.chars().count())
        .sum::<usize>()
        .max(1);
    let mut out = Vec::new();
    let mut index = 0;
    for (text, style) in segments {
        for ch in text.chars() {
            let t = if total > 1 {
                index as f32 / (total - 1) as f32
            } else {
                0.0
            };
            out.push((ch.to_string(), style.fg(color_mix(start, end, t))));
            index += 1;
        }
    }
    out
}

fn horizontal_rule(width: usize, theme: &Theme) -> Line<'static> {
    if width < 12 {
        return gradient_line(width, "─", theme.color("overlay0"), theme.color("surface2"));
    }
    let left = (width - 3) / 2;
    let right = width - 3 - left;
    let mut spans = gradient_chars(left, "─", theme.color("overlay0"), theme.color("surface2"));
    spans.push(Span::styled(
        " ◆ ",
        Style::default()
            .fg(theme.color("pink"))
            .add_modifier(Modifier::BOLD),
    ));
    spans.extend(gradient_chars(
        right,
        "─",
        theme.color("surface2"),
        theme.color("overlay0"),
    ));
    Line::from(spans)
}

fn wrap_styled(
    segments: &[(String, Style)],
    width: usize,
    max_lines: usize,
) -> Vec<Vec<Span<'static>>> {
    let width = width.max(4);
    let mut lines: Vec<Vec<Span<'static>>> = vec![Vec::new()];
    let mut used = 0usize;
    for (text, style) in segments {
        let mut chunk = String::new();
        let mut space: Option<bool> = None;
        for ch in text.chars() {
            let is_space = ch == ' ' || ch == '\t';
            if space.is_some_and(|state| state != is_space) {
                emit_chunk(
                    &mut lines,
                    &mut used,
                    &mut chunk,
                    space == Some(true),
                    *style,
                    width,
                );
            }
            if chunk.is_empty() {
                space = Some(is_space);
            }
            chunk.push(ch);
        }
        if !chunk.is_empty() {
            emit_chunk(
                &mut lines,
                &mut used,
                &mut chunk,
                space == Some(true),
                *style,
                width,
            );
        }
    }
    lines.truncate(max_lines.max(1));
    lines
}

fn emit_chunk(
    lines: &mut Vec<Vec<Span<'static>>>,
    used: &mut usize,
    chunk: &mut String,
    is_space: bool,
    style: Style,
    width: usize,
) {
    let text = std::mem::take(chunk);
    let chunk_width = format::text_width(&text);
    if is_space {
        if *used == 0 || *used + chunk_width > width {
            return;
        }
        lines.last_mut().unwrap().push(Span::styled(text, style));
        *used += chunk_width;
        return;
    }
    if *used > 0 && *used + chunk_width > width {
        lines.push(Vec::new());
        *used = 0;
    }
    if chunk_width > width {
        for ch in text.chars() {
            let char_width = format::text_width(&ch.to_string());
            if *used + char_width > width && *used > 0 {
                lines.push(Vec::new());
                *used = 0;
            }
            lines
                .last_mut()
                .unwrap()
                .push(Span::styled(ch.to_string(), style));
            *used += char_width;
        }
        return;
    }
    lines.last_mut().unwrap().push(Span::styled(text, style));
    *used += chunk_width;
}

fn prefixed_lines(
    prefix: &[Span<'static>],
    segments: &[(String, Style)],
    width: usize,
    max_lines: usize,
) -> Vec<Line<'static>> {
    let prefix_width = prefix
        .iter()
        .map(|span| format::text_width(&span.content))
        .sum::<usize>();
    let body_width = width.saturating_sub(prefix_width).max(4);
    let wrapped = wrap_styled(segments, body_width, max_lines);
    let indent = " ".repeat(prefix_width);
    wrapped
        .into_iter()
        .enumerate()
        .map(|(row, spans)| {
            let mut line = if row == 0 {
                prefix.to_vec()
            } else {
                vec![Span::raw(indent.clone())]
            };
            line.extend(spans);
            Line::from(line)
        })
        .collect()
}

fn code_segments(line: &str, lang: &str, theme: &Theme) -> Vec<(String, Style)> {
    let lang = lang.to_ascii_lowercase();
    let comment = match lang.as_str() {
        "sh" | "bash" | "zsh" | "py" | "python" | "toml" | "yaml" | "yml" | "ruby" => "#",
        "sql" | "lua" | "haskell" => "--",
        _ => "//",
    };
    let base = Style::default().fg(theme.color("subtext1"));
    let comment_style = base
        .fg(theme.color("overlay0"))
        .add_modifier(Modifier::ITALIC);
    let string_style = base.fg(theme.color("green"));
    let number_style = base.fg(theme.color("peach"));
    let keyword_style = base.fg(theme.color("mauve")).add_modifier(Modifier::BOLD);
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<(String, Style)> = Vec::new();
    let mut plain = String::new();
    let mut index = 0;
    while index < chars.len() {
        if starts_with(&chars, index, comment) {
            flush_plain(&mut out, &mut plain, base);
            let rest: String = chars[index..].iter().collect();
            out.push((rest, comment_style));
            break;
        }
        let ch = chars[index];
        if ch == '"' || ch == '\'' || ch == '`' {
            flush_plain(&mut out, &mut plain, base);
            let mut cursor = index + 1;
            let mut text = String::new();
            text.push(ch);
            while cursor < chars.len() {
                text.push(chars[cursor]);
                if chars[cursor] == '\\' && cursor + 1 < chars.len() {
                    cursor += 1;
                    text.push(chars[cursor]);
                } else if chars[cursor] == ch {
                    cursor += 1;
                    break;
                }
                cursor += 1;
            }
            out.push((text, string_style));
            index = cursor;
            continue;
        }
        if ch.is_ascii_digit() && (index == 0 || !chars[index - 1].is_alphanumeric()) {
            flush_plain(&mut out, &mut plain, base);
            let mut cursor = index;
            let mut text = String::new();
            while cursor < chars.len()
                && (chars[cursor].is_ascii_alphanumeric() || matches!(chars[cursor], '.' | '_'))
            {
                text.push(chars[cursor]);
                cursor += 1;
            }
            out.push((text, number_style));
            index = cursor;
            continue;
        }
        if ch.is_alphabetic() || ch == '_' {
            flush_plain(&mut out, &mut plain, base);
            let mut cursor = index;
            let mut word = String::new();
            while cursor < chars.len() && (chars[cursor].is_alphanumeric() || chars[cursor] == '_')
            {
                word.push(chars[cursor]);
                cursor += 1;
            }
            let style = if is_code_keyword(&word.to_ascii_lowercase()) {
                keyword_style
            } else {
                base
            };
            out.push((word, style));
            index = cursor;
            continue;
        }
        plain.push(ch);
        index += 1;
    }
    flush_plain(&mut out, &mut plain, base);
    if out.is_empty() {
        out.push((String::new(), base));
    }
    out
}

fn task_check(line: &str) -> Option<bool> {
    let line = line.trim_start();
    for (prefix, done) in [
        ("- [x] ", true),
        ("* [x] ", true),
        ("- [X] ", true),
        ("* [X] ", true),
        ("- [ ] ", false),
        ("* [ ] ", false),
    ] {
        if line.starts_with(prefix) {
            return Some(done);
        }
    }
    None
}

fn task_progress_line(done: usize, total: usize, width: usize, theme: &Theme) -> Line<'static> {
    if width < 24 {
        return Line::from(vec![
            Span::styled(
                format!("{} ", icons::CHECK_SQUARE),
                Style::default().fg(theme.color("green")),
            ),
            Span::styled(
                format!("任务 {done}/{total}"),
                Style::default().fg(theme.color("subtext0")),
            ),
        ]);
    }
    let bar = width.saturating_sub(22).clamp(4, 20);
    let filled = (done * bar).checked_div(total).unwrap_or(0);
    let mut spans = vec![
        Span::styled(
            format!("{} ", icons::CHECK_SQUARE),
            Style::default().fg(theme.color("green")),
        ),
        Span::styled("任务 ", Style::default().fg(theme.color("subtext0"))),
        Span::styled(
            format!("{done}/{total} "),
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
    ];
    spans.extend(gradient_chars(
        filled,
        "█",
        theme.color("green"),
        theme.color("teal"),
    ));
    spans.push(Span::styled(
        "░".repeat(bar - filled),
        Style::default().fg(theme.color("surface2")),
    ));
    let percent = (done * 100).checked_div(total).unwrap_or(0);
    spans.push(Span::styled(
        format!(" {percent}%"),
        Style::default().fg(theme.color("overlay0")),
    ));
    Line::from(spans)
}

fn code_accent(lang: &str, theme: &Theme) -> Color {
    match lang.to_ascii_lowercase().as_str() {
        "rust" => theme.color("peach"),
        "py" | "python" => theme.color("blue"),
        "js" | "javascript" | "ts" | "typescript" => theme.color("yellow"),
        "sh" | "bash" | "zsh" | "console" => theme.color("green"),
        "json" | "toml" | "yaml" | "yml" => theme.color("lavender"),
        "sql" => theme.color("mauve"),
        "html" | "css" => theme.color("sapphire"),
        _ => theme.color("blue"),
    }
}

fn is_code_keyword(word: &str) -> bool {
    matches!(
        word,
        "fn" | "let"
            | "mut"
            | "pub"
            | "use"
            | "mod"
            | "struct"
            | "enum"
            | "impl"
            | "trait"
            | "for"
            | "while"
            | "loop"
            | "if"
            | "else"
            | "match"
            | "return"
            | "self"
            | "async"
            | "await"
            | "move"
            | "where"
            | "const"
            | "static"
            | "type"
            | "def"
            | "class"
            | "import"
            | "from"
            | "as"
            | "try"
            | "except"
            | "finally"
            | "with"
            | "lambda"
            | "pass"
            | "raise"
            | "in"
            | "is"
            | "not"
            | "and"
            | "or"
            | "none"
            | "true"
            | "false"
            | "null"
            | "nil"
            | "function"
            | "var"
            | "new"
            | "this"
            | "throw"
            | "catch"
            | "export"
            | "default"
            | "interface"
            | "extends"
            | "package"
            | "func"
            | "go"
            | "defer"
            | "select"
            | "case"
            | "switch"
            | "break"
            | "continue"
            | "echo"
            | "cd"
            | "local"
            | "then"
            | "fi"
            | "done"
            | "do"
            | "esac"
            | "insert"
            | "update"
            | "delete"
    )
}

fn heading_level(line: &str) -> Option<usize> {
    let level = line.chars().take_while(|ch| *ch == '#').count();
    ((1..=6).contains(&level) && line[level..].starts_with(' ')).then_some(level)
}

fn is_table_start(lines: &[&str], index: usize) -> bool {
    lines[index].contains('|')
        && lines
            .get(index + 1)
            .is_some_and(|next| is_table_separator(next))
}

fn is_block_start(lines: &[&str], index: usize) -> bool {
    let candidate = lines[index].trim();
    candidate.is_empty()
        || candidate.starts_with("<!--")
        || candidate.starts_with('>')
        || heading_level(candidate).is_some()
        || candidate.starts_with("```")
        || candidate.starts_with("~~~")
        || is_horizontal_rule(candidate)
        || list_marker(candidate).is_some()
        || is_table_start(lines, index)
}

pub(crate) fn markdown_lines(text: &str, width: usize, theme: &Theme) -> Vec<Line<'static>> {
    let width = width.max(10);
    let mut out: Vec<Line<'static>> = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut done = 0;
    let mut total = 0;
    for line in &lines {
        if let Some(state) = task_check(line) {
            total += 1;
            done += usize::from(state);
        }
    }
    let mut task_banner = false;
    let mut index = 0;
    while index < lines.len() && out.len() < 4000 {
        let raw = lines[index].trim_end();
        let trimmed = raw.trim_start();
        let indent = raw.len().saturating_sub(trimmed.len());

        if trimmed.is_empty() {
            if out.last().is_some_and(|line| line.width() > 0) {
                out.push(Line::default());
            }
            index += 1;
            continue;
        }
        if trimmed.starts_with("<!--") {
            while index < lines.len() && !lines[index].contains("-->") {
                index += 1;
            }
            index += 1;
            continue;
        }
        let fence = if trimmed.starts_with("```") {
            Some("```")
        } else if trimmed.starts_with("~~~") {
            Some("~~~")
        } else {
            None
        };
        if let Some(fence) = fence {
            let lang = trimmed.trim_start_matches(['`', '~']).trim().to_string();
            let accent = code_accent(&lang, theme);
            let label = if lang.is_empty() {
                " code ".to_string()
            } else {
                format!(" {lang} ")
            };
            let label = format::truncate(&label, width.saturating_sub(2));
            let label_width = format::text_width(&label);
            let card_bg = Style::default().bg(theme.color("surface0"));
            let mut top = vec![
                Span::styled("╭", Style::default().fg(theme.color("surface2"))),
                Span::styled(label, card_bg.fg(accent).add_modifier(Modifier::BOLD)),
            ];
            top.extend(gradient_chars(
                width.saturating_sub(label_width + 2),
                "─",
                accent,
                theme.color("mauve"),
            ));
            top.push(Span::styled(
                "╮",
                Style::default().fg(theme.color("surface2")),
            ));
            out.push(Line::from(top));
            index += 1;
            let mut count = 0;
            while index < lines.len() && !lines[index].trim_start().starts_with(fence) {
                let source = lines[index].replace('\t', "    ");
                let mut segments = code_segments(&source, &lang, theme);
                for (_, style) in segments.iter_mut() {
                    *style = style.bg(theme.color("surface0"));
                }
                for chunk in wrap_styled(&segments, width.saturating_sub(3), 40) {
                    let mut spans = vec![
                        Span::styled("│", card_bg.fg(accent)),
                        Span::styled(" ", card_bg),
                    ];
                    let mut used = 2usize;
                    for span in chunk {
                        used += format::text_width(&span.content);
                        spans.push(span);
                    }
                    spans.push(Span::styled(
                        " ".repeat(width.saturating_sub(used + 1)),
                        card_bg,
                    ));
                    spans.push(Span::styled("│", card_bg.fg(theme.color("surface2"))));
                    out.push(Line::from(spans));
                }
                index += 1;
                count += 1;
                if count >= 300 || out.len() >= 4000 {
                    break;
                }
            }
            if index < lines.len() && lines[index].trim_start().starts_with(fence) {
                index += 1;
            }
            let mut bottom = vec![Span::styled(
                "╰",
                Style::default().fg(theme.color("surface2")),
            )];
            bottom.extend(gradient_chars(
                width.saturating_sub(2),
                "─",
                accent,
                theme.color("mauve"),
            ));
            bottom.push(Span::styled(
                "╯",
                Style::default().fg(theme.color("surface2")),
            ));
            out.push(Line::from(bottom));
            out.push(Line::default());
            continue;
        }
        if trimmed.contains('|')
            && lines
                .get(index + 1)
                .is_some_and(|next| is_table_separator(next))
        {
            let mut rows = Vec::new();
            while index < lines.len()
                && lines[index].contains('|')
                && !lines[index].trim().is_empty()
            {
                rows.push(lines[index]);
                index += 1;
            }
            out.extend(render_markdown_table(&rows, width, theme));
            out.push(Line::default());
            continue;
        }
        if is_horizontal_rule(trimmed) {
            out.push(horizontal_rule(width, theme));
            index += 1;
            continue;
        }
        if trimmed.starts_with('>') {
            let mut quote = Vec::new();
            while index < lines.len() && lines[index].trim_start().starts_with('>') {
                let content = lines[index]
                    .trim_start()
                    .trim_start_matches('>')
                    .trim_start()
                    .to_string();
                quote.push(content);
                index += 1;
            }
            let first = quote.first().cloned().unwrap_or_default();
            let callout = if first.starts_with("[!") && first.contains(']') {
                let end = first.find(']').unwrap_or(2);
                let label = first[2..end].to_ascii_uppercase();
                let rest = first[end + 1..].trim().to_string();
                Some((label, rest))
            } else {
                None
            };
            let style = Style::default()
                .fg(theme.color("overlay1"))
                .add_modifier(Modifier::ITALIC);
            let (prefix, body) = if let Some((label, rest)) = callout {
                let (icon, color) = match label.as_str() {
                    "NOTE" => (icons::INFO, theme.color("blue")),
                    "TIP" => (icons::FLAME, theme.color("green")),
                    "IMPORTANT" => (icons::WARNING, theme.color("mauve")),
                    "WARNING" => (icons::WARNING, theme.color("yellow")),
                    _ => (icons::WARNING, theme.color("red")),
                };
                let prefix = vec![Span::styled(
                    "┃ ",
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )];
                out.push(Line::from(vec![
                    prefix[0].clone(),
                    Span::styled(
                        format!(" {icon} {label} "),
                        Style::default()
                            .fg(theme.color("base"))
                            .bg(color)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                if !rest.is_empty() {
                    let segments = inline_segments(&rest, style, theme);
                    out.extend(prefixed_lines(&prefix, &segments, width, 10));
                }
                quote.remove(0);
                (prefix, quote)
            } else {
                (
                    vec![Span::styled(
                        "▎ ",
                        Style::default().fg(theme.color("overlay0")),
                    )],
                    quote,
                )
            };
            for line in body.iter().filter(|line| !line.is_empty()) {
                let segments = inline_segments(line, style, theme);
                out.extend(prefixed_lines(&prefix, &segments, width, 30));
            }
            out.push(Line::default());
            continue;
        }
        if let Some(level) = heading_level(trimmed) {
            let text = trimmed[level..].trim().trim_end_matches('#').trim_end();
            let (start, end, prefix, rule) = match level {
                1 => (
                    theme.color("blue"),
                    theme.color("mauve"),
                    vec![Span::styled(
                        "▌ ",
                        Style::default()
                            .fg(theme.color("blue"))
                            .add_modifier(Modifier::BOLD),
                    )],
                    Some(("━", width.min(90), true)),
                ),
                2 => (
                    theme.color("lavender"),
                    theme.color("teal"),
                    vec![Span::styled(
                        "▎ ",
                        Style::default().fg(theme.color("mauve")),
                    )],
                    Some(("─", width.min(60), false)),
                ),
                3 => (
                    theme.color("peach"),
                    theme.color("yellow"),
                    vec![Span::styled(
                        "▍ ",
                        Style::default().fg(theme.color("peach")),
                    )],
                    None,
                ),
                _ => (
                    theme.color("subtext0"),
                    theme.color("overlay1"),
                    vec![
                        Span::raw("  "),
                        Span::styled("· ", Style::default().fg(theme.color("overlay0"))),
                    ],
                    None,
                ),
            };
            let style = Style::default().fg(start).add_modifier(if level <= 3 {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });
            let mut segments = inline_segments(text, style, theme);
            if level <= 3 {
                segments = gradient_segments(&segments, start, end);
            }
            out.extend(prefixed_lines(&prefix, &segments, width, 8));
            if let Some((glyph, rule_width, strong)) = rule {
                if strong {
                    out.push(gradient_line(rule_width, glyph, start, end));
                } else {
                    out.push(gradient_line(
                        rule_width,
                        glyph,
                        theme.color("surface2"),
                        theme.color("overlay0"),
                    ));
                }
            } else {
                out.push(Line::default());
            }
            index += 1;
            continue;
        }
        if let Some((marker, rest)) = list_marker(trimmed) {
            let level = (indent / 2).min(6);
            let mut prefix = vec![Span::raw("  ".repeat(level))];
            let marker_text = marker.trim();
            let task = marker_text == "[ ]" || marker_text == "[x]";
            let text_style = if task {
                if !task_banner {
                    out.push(task_progress_line(done, total, width, theme));
                    task_banner = true;
                }
                let done = marker_text == "[x]";
                prefix.push(Span::styled(
                    format!(
                        "{} ",
                        if done {
                            icons::CHECK_SQUARE
                        } else {
                            icons::SQUARE
                        }
                    ),
                    Style::default().fg(if done {
                        theme.color("green")
                    } else {
                        theme.color("overlay0")
                    }),
                ));
                if done {
                    Style::default()
                        .fg(theme.color("overlay0"))
                        .add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default().fg(theme.color("subtext0"))
                }
            } else {
                let color = match level + 1 {
                    1 => theme.color("blue"),
                    2 => theme.color("mauve"),
                    3 => theme.color("peach"),
                    _ => theme.color("overlay1"),
                };
                if marker_text == "•" {
                    let glyph = match level {
                        0 => "●",
                        1 => "○",
                        2 => "▪",
                        _ => "·",
                    };
                    prefix.push(Span::styled(
                        format!("{glyph} "),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ));
                } else {
                    prefix.push(Span::styled(
                        marker,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ));
                }
                Style::default().fg(theme.color("subtext0"))
            };
            let segments = inline_segments(rest, text_style, theme);
            out.extend(prefixed_lines(&prefix, &segments, width, 40));
            index += 1;
            continue;
        }

        let mut paragraph = String::new();
        while index < lines.len() {
            let first = paragraph.is_empty();
            if !first && is_block_start(&lines, index) {
                break;
            }
            let candidate = lines[index].trim();
            if !first {
                paragraph.push(' ');
            }
            paragraph.push_str(candidate);
            index += 1;
        }
        let segments = inline_segments(
            &paragraph,
            Style::default().fg(theme.color("subtext0")),
            theme,
        );
        if segments.iter().all(|(text, _)| text.trim().is_empty()) {
            continue;
        }
        out.extend(prefixed_lines(&[], &segments, width, 60));
    }
    if out.len() >= 4000 && index < lines.len() {
        out.push(Line::default());
        out.push(Line::from(Span::styled(
            "…… 内容太长啦，后面省略喵~",
            Style::default()
                .fg(theme.color("overlay0"))
                .add_modifier(Modifier::ITALIC),
        )));
    }
    if out.is_empty() {
        out.push(Line::from("这里空空的，人家什么也没读到喵~"));
    }
    out
}

fn list_marker(line: &str) -> Option<(String, &str)> {
    for (marker, prefix) in [
        ("- [ ] ", "[ ] "),
        ("- [x] ", "[x] "),
        ("- [X] ", "[x] "),
        ("* [ ] ", "[ ] "),
        ("* [x] ", "[x] "),
        ("- ", "• "),
        ("* ", "• "),
        ("+ ", "• "),
    ] {
        if let Some(rest) = line.strip_prefix(marker) {
            return Some((prefix.to_string(), rest));
        }
    }
    let digits = line.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digits > 0
        && digits <= 3
        && matches!(line.as_bytes().get(digits), Some(b'.') | Some(b')'))
        && line.as_bytes().get(digits + 1) == Some(&b' ')
    {
        return Some((
            format!("{}. ", &line[..digits]),
            line[digits + 2..].trim_start(),
        ));
    }
    None
}

fn is_horizontal_rule(line: &str) -> bool {
    let compact: String = line.chars().filter(|ch| !ch.is_whitespace()).collect();
    compact.len() >= 3
        && compact.chars().all(|ch| matches!(ch, '-' | '*' | '_'))
        && compact
            .chars()
            .next()
            .is_some_and(|first| compact.chars().all(|ch| ch == first))
}

fn is_table_separator(line: &str) -> bool {
    let cells = split_table_row(line);
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let cell = cell.trim();
            cell.contains('-') && cell.chars().all(|ch| ch == '-' || ch == ':')
        })
}

fn table_alignments(line: &str) -> Vec<Alignment> {
    split_table_row(line)
        .iter()
        .map(|cell| {
            let cell = cell.trim();
            if cell.starts_with(':') && cell.ends_with(':') {
                Alignment::Center
            } else if cell.ends_with(':') {
                Alignment::Right
            } else {
                Alignment::Left
            }
        })
        .collect()
}

fn split_table_row(line: &str) -> Vec<String> {
    let line = line.trim();
    let line = line.strip_prefix('|').unwrap_or(line);
    let line = line.strip_suffix('|').unwrap_or(line);
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut in_code = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' if chars.peek() == Some(&'|') => {
                chars.next();
                cell.push('|');
            }
            '`' => {
                in_code = !in_code;
                cell.push(ch);
            }
            '|' if !in_code => {
                cells.push(strip_markdown_inline(cell.trim()));
                cell.clear();
            }
            _ => cell.push(ch),
        }
    }
    cells.push(strip_markdown_inline(cell.trim()));
    cells
}

fn pad_table_cell(text: &str, width: usize, align: Alignment) -> String {
    let text = format::truncate(text, width);
    let pad = width.saturating_sub(format::text_width(&text));
    match align {
        Alignment::Right => format!("{}{text}", " ".repeat(pad)),
        Alignment::Center => {
            let left = pad / 2;
            format!("{}{text}{}", " ".repeat(left), " ".repeat(pad - left))
        }
        _ => format!("{text}{}", " ".repeat(pad)),
    }
}

fn render_markdown_table(rows: &[&str], width: usize, theme: &Theme) -> Vec<Line<'static>> {
    if rows.len() < 2 {
        return Vec::new();
    }
    let header = split_table_row(rows[0]);
    let aligns = table_alignments(rows[1]);
    let body: Vec<Vec<String>> = rows
        .iter()
        .skip(2)
        .map(|row| split_table_row(row))
        .collect();
    let columns = header
        .len()
        .max(body.iter().map(Vec::len).max().unwrap_or(0));
    if columns == 0 {
        return Vec::new();
    }
    if width < 3 * columns + 1 {
        return rows
            .iter()
            .enumerate()
            .filter(|(row, _)| *row != 1)
            .flat_map(|(_, row)| {
                let text = split_table_row(row).join(" · ");
                format::wrap_text(&text, width, 12)
                    .into_iter()
                    .map(Line::from)
                    .collect::<Vec<_>>()
            })
            .collect();
    }
    let mut widths: Vec<usize> = (0..columns)
        .map(|column| {
            std::iter::once(
                header
                    .get(column)
                    .map(|cell| format::text_width(cell))
                    .unwrap_or(0),
            )
            .chain(body.iter().map(|row| {
                row.get(column)
                    .map(|cell| format::text_width(cell))
                    .unwrap_or(0)
            }))
            .max()
            .unwrap_or(1)
            .clamp(2, 48)
        })
        .collect();
    let mut total = 1 + widths.iter().map(|cell| cell + 3).sum::<usize>();
    while total > width {
        let Some((slot, _)) = widths
            .iter()
            .enumerate()
            .filter(|(_, cell)| **cell > 2)
            .max_by_key(|(_, cell)| **cell)
        else {
            break;
        };
        widths[slot] -= 1;
        total -= 1;
    }

    let border_style = Style::default().fg(theme.color("overlay0"));
    let header_style = Style::default()
        .fg(theme.color("blue"))
        .bg(theme.color("surface0"))
        .add_modifier(Modifier::BOLD);
    let cell_style = Style::default().fg(theme.color("subtext0"));
    let border = |left: &str, middle: &str, right: &str| {
        let mut text = String::from(left);
        for (column, cell) in widths.iter().enumerate() {
            text.push_str(&"─".repeat(cell + 2));
            text.push_str(if column + 1 == widths.len() {
                right
            } else {
                middle
            });
        }
        Line::from(Span::styled(text, border_style))
    };
    let render_row = |cells: &[String], align: &[Alignment], style: Style| {
        let wrapped: Vec<Vec<String>> = (0..columns)
            .map(|column| {
                let text = cells.get(column).cloned().unwrap_or_default();
                let chunks = format::wrap_text(&text, widths[column], 8);
                if chunks.is_empty() {
                    vec![String::new()]
                } else {
                    chunks
                }
            })
            .collect();
        let height = wrapped.iter().map(Vec::len).max().unwrap_or(1);
        let mut lines = Vec::new();
        for row in 0..height {
            let mut spans = vec![Span::styled("│", border_style)];
            for column in 0..columns {
                let text = wrapped[column].get(row).cloned().unwrap_or_default();
                let alignment = align.get(column).copied().unwrap_or(Alignment::Left);
                spans.push(Span::styled(
                    format!(" {} ", pad_table_cell(&text, widths[column], alignment)),
                    style,
                ));
                spans.push(Span::styled("│", border_style));
            }
            lines.push(Line::from(spans));
        }
        lines
    };

    let mut out = vec![border("╭", "┬", "╮")];
    let header_align = aligns.clone();
    out.extend(render_row(&header, &header_align, header_style));
    out.push(border("├", "┼", "┤"));
    for (index, row) in body.iter().enumerate() {
        let style = if index % 2 == 1 {
            cell_style.bg(theme.color("surface0"))
        } else {
            cell_style
        };
        out.extend(render_row(row, &aligns, style));
    }
    out.push(border("╰", "┴", "╯"));
    out
}

fn strip_markdown_images(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '!' && chars.get(index + 1) == Some(&'[') {
            let mut cursor = index + 2;
            let mut depth = 1;
            while cursor < chars.len() && depth > 0 {
                match chars[cursor] {
                    '[' => depth += 1,
                    ']' => depth -= 1,
                    _ => {}
                }
                cursor += 1;
            }
            if chars.get(cursor) == Some(&'(') {
                cursor += 1;
                let mut paren = 1;
                while cursor < chars.len() && paren > 0 {
                    match chars[cursor] {
                        '(' => paren += 1,
                        ')' => paren -= 1,
                        _ => {}
                    }
                    cursor += 1;
                }
            }
            index = cursor;
            continue;
        }
        out.push(chars[index]);
        index += 1;
    }
    out
}

pub(crate) fn strip_markdown_inline(text: &str) -> String {
    let text = strip_markdown_images(text);
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '!' if chars.peek() == Some(&'[') => {
                chars.next();
                let mut alt = String::new();
                let mut depth = 1;
                for inner in chars.by_ref() {
                    if inner == '[' {
                        depth += 1;
                    } else if inner == ']' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    alt.push(inner);
                }
                if chars.peek() == Some(&'(') {
                    chars.next();
                    for inner in chars.by_ref() {
                        if inner == ')' {
                            break;
                        }
                    }
                }
                out.push_str(&alt);
            }
            '[' => {
                let mut label = String::new();
                let mut found = false;
                for inner in chars.by_ref() {
                    if inner == ']' {
                        found = true;
                        break;
                    }
                    label.push(inner);
                }
                if found && chars.peek() == Some(&'(') {
                    chars.next();
                    for inner in chars.by_ref() {
                        if inner == ')' {
                            break;
                        }
                    }
                }
                out.push_str(&label);
            }
            '*' | '_' | '`' | '~' => {}
            '<' => {
                for inner in chars.by_ref() {
                    if inner == '>' {
                        break;
                    }
                }
            }
            _ => out.push(ch),
        }
    }
    let cleaned = out.trim().to_string();
    format::clean_text(&cleaned)
}

fn draw_settings_page(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    fill(frame, area, theme.color("base"));
    let width = area.width.saturating_sub(8).min(100);
    let left = area.x + (area.width.saturating_sub(width)) / 2;
    let inner = Rect::new(left, area.y + 1, width, area.height.saturating_sub(2));
    app.layout.settings = inner;
    frame.render_widget(
        Paragraph::new(icons::label(icons::SETTINGS, "设置喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new("想改什么就点一点，改完记得让人家保存哦喵~")
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let actions_y = inner.y + inner.height.saturating_sub(2);
    let mut y = inner.y + 2;

    frame.render_widget(
        Paragraph::new(format!(
            "{} 配置文件  {}喵~",
            icons::TERMINAL,
            format::truncate(
                &app.config_file_label(),
                inner.width.saturating_sub(14) as usize
            )
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 2;

    if y + 5 < actions_y {
        let theme_header = icons::label(icons::PALETTE, "主题喵~");
        let theme_offset = format::text_width(&theme_header) as u16 + 2;
        frame.render_widget(
            Paragraph::new(theme_header).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(inner.x, y, inner.width, 1),
        );
        frame.render_widget(
            Paragraph::new(
                "Catppuccin 四套和自定义主题，点一下就能换喵~ 自定义放 ~/.config/gitx-tui/themes/",
            )
            .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                inner.x + theme_offset,
                y,
                inner.width.saturating_sub(theme_offset),
                1,
            ),
        );
        y += 1;
        let themes = app.themes.clone();
        let gap = 1_u16;
        let columns = 4_u16;
        let card_width = (inner.width.saturating_sub(gap * (columns - 1))) / columns;
        let mut index = 0;
        while index < themes.len() && y + 4 <= actions_y && card_width >= 12 {
            for column in 0..columns {
                if index >= themes.len() {
                    break;
                }
                let rect = Rect::new(inner.x + column * (card_width + gap), y, card_width, 4);
                if rect.x + rect.width > inner.x + inner.width {
                    break;
                }
                draw_theme_card(frame, app, theme, rect, &themes[index], hits);
                index += 1;
            }
            y += 5;
        }
    }

    if y + 6 < actions_y {
        let gap = 2_u16;
        let column_width = (inner.width.saturating_sub(gap)) / 2;
        let left_column = Rect::new(inner.x, y, column_width, actions_y - y);
        let right_column = Rect::new(
            inner.x + column_width + gap,
            y,
            inner.width.saturating_sub(column_width + gap),
            actions_y - y,
        );

        let preview_header = icons::label(icons::BOOK, "仓库预览喵~");
        let preview_offset = format::text_width(&preview_header) as u16 + 2;
        frame.render_widget(
            Paragraph::new(preview_header).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(left_column.x, y, left_column.width, 1),
        );
        frame.render_widget(
            Paragraph::new("默认关闭，人家不会偷偷去取喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                left_column.x + preview_offset,
                y,
                left_column.width.saturating_sub(preview_offset),
                1,
            ),
        );
        draw_toggle_row(
            frame,
            app,
            theme,
            hits,
            Rect::new(left_column.x, y + 1, left_column.width, 1),
            1,
            "文件树预览喵~",
            app.config.show_file_tree,
            Action::ToggleFileTree,
        );
        draw_toggle_row(
            frame,
            app,
            theme,
            hits,
            Rect::new(left_column.x, y + 2, left_column.width, 1),
            2,
            "README 预览喵~",
            app.config.show_readme,
            Action::ToggleReadme,
        );
        frame.render_widget(
            Paragraph::new(icons::label(icons::DOWNLOAD, "下载喵~")).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(left_column.x, y + 4, left_column.width, 1),
        );
        draw_download_dir_row(
            frame,
            app,
            theme,
            hits,
            Rect::new(left_column.x, y + 5, left_column.width, 1),
        );
        let open_dir = icons::label(icons::DOWNLOAD_DIR, "打开下载目录喵~");
        let open_width = format::text_width(&open_dir) as u16 + 4;
        if y + 6 < actions_y {
            draw_button(
                frame,
                app,
                theme,
                hits,
                Rect::new(left_column.x, y + 6, open_width, 1),
                &open_dir,
                Action::OpenDownloadDir,
                false,
            );
            frame.render_widget(
                Paragraph::new("同名文件会自动改名，不会覆盖旧的哦喵~")
                    .style(Style::default().fg(theme.color("overlay0"))),
                Rect::new(left_column.x, y + 7, left_column.width.saturating_sub(2), 1),
            );
        }

        let network_header = icons::label(icons::NETWORK, "网络喵~");
        let network_offset = format::text_width(&network_header) as u16 + 2;
        frame.render_widget(
            Paragraph::new(network_header).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(right_column.x, y, right_column.width, 1),
        );
        frame.render_widget(
            Paragraph::new("公开接口先走代理，认证请求直连 GitHub 哦喵~")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                right_column.x + network_offset,
                y,
                right_column.width.saturating_sub(network_offset),
                1,
            ),
        );
        draw_proxy_row(
            frame,
            app,
            theme,
            hits,
            Rect::new(right_column.x, y + 1, right_column.width, 1),
        );
        let cache_header = icons::label(icons::DATABASE, "缓存喵~");
        let cache_offset = format::text_width(&cache_header) as u16 + 2;
        frame.render_widget(
            Paragraph::new(cache_header).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(right_column.x, y + 3, right_column.width, 1),
        );
        frame.render_widget(
            Paragraph::new(format!(
                "{} 个文件 · {}喵~",
                app.cache_stats.files,
                format::format_size(app.cache_stats.bytes)
            ))
            .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(
                right_column.x + cache_offset,
                y + 3,
                right_column.width.saturating_sub(cache_offset),
                1,
            ),
        );
        for (index, (label, kind)) in [
            ("仓库列表喵~", TtlKind::Repositories),
            ("详情与发行版喵~", TtlKind::Detail),
            ("搜索结果喵~", TtlKind::Search),
        ]
        .into_iter()
        .enumerate()
        {
            if y + 4 + index as u16 >= actions_y {
                break;
            }
            draw_ttl_row(
                frame,
                app,
                theme,
                hits,
                Rect::new(right_column.x, y + 4 + index as u16, right_column.width, 1),
                label,
                kind,
                5 + index,
            );
        }
        if y + 7 < actions_y {
            draw_commit_limit_row(
                frame,
                app,
                theme,
                hits,
                Rect::new(right_column.x, y + 7, right_column.width, 1),
            );
        }
        if y + 8 < actions_y {
            draw_default_page_row(
                frame,
                app,
                theme,
                hits,
                Rect::new(right_column.x, y + 8, right_column.width, 1),
            );
        }
    }

    let mut x = inner.x;
    let save_action = Action::SaveConfig;
    let save_hover = app.hover.as_ref() == Some(&save_action);
    let save_width = 18;
    fill(
        frame,
        Rect::new(x, actions_y, save_width, 1),
        if save_hover {
            theme.color("sapphire")
        } else {
            theme.color("blue")
        },
    );
    frame.render_widget(
        Paragraph::new(format!("{} 保存设置喵~", icons::SAVE))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(theme.color("base"))
                    .add_modifier(Modifier::BOLD),
            ),
        Rect::new(x, actions_y, save_width, 1),
    );
    register(hits, Rect::new(x, actions_y, save_width, 1), save_action);
    x += save_width + 1;
    let clear_action = Action::ClearCache;
    let clear_hover = app.hover.as_ref() == Some(&clear_action);
    let clear_width = 16;
    fill(
        frame,
        Rect::new(x, actions_y, clear_width, 1),
        if clear_hover {
            theme.color("surface1")
        } else {
            theme.color("surface0")
        },
    );
    frame.render_widget(
        Paragraph::new(format!("{} 清掉缓存喵~", icons::TRASH))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.color("red"))),
        Rect::new(x, actions_y, clear_width, 1),
    );
    register(hits, Rect::new(x, actions_y, clear_width, 1), clear_action);
    x += clear_width + 1;
    let manage_label = icons::label(icons::DATABASE, "缓存管理喵~");
    let manage_width = format::text_width(&manage_label) as u16 + 4;
    if x + manage_width <= inner.x + inner.width {
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, actions_y, manage_width, 1),
            &manage_label,
            Action::OpenCacheManager,
            false,
        );
    }
    if app.config_dirty {
        frame.render_widget(
            Paragraph::new("有改动还没保存呢喵~")
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(inner.x, actions_y, inner.width, 1),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_toggle_row(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    rect: Rect,
    row: usize,
    label: &str,
    enabled: bool,
    action: Action,
) {
    if app.settings_row == row {
        fill(frame, rect, theme.color("surface0"));
    }
    let icon = if enabled {
        icons::CHECK_SQUARE
    } else {
        icons::SQUARE
    };
    frame.render_widget(
        Paragraph::new(format!("{icon} {label}")).style(if enabled {
            Style::default()
                .fg(theme.color("green"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.color("subtext1"))
        }),
        Rect::new(rect.x + 1, rect.y, rect.width.saturating_sub(12), 1),
    );
    frame.render_widget(
        Paragraph::new(if enabled {
            "已开启喵~"
        } else {
            "已关闭喵~"
        })
        .alignment(Alignment::Right)
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(rect.x, rect.y, rect.width.saturating_sub(1), 1),
    );
    register(hits, rect, action);
}

fn draw_download_dir_row(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    rect: Rect,
) {
    if app.settings_row == 3 {
        fill(frame, rect, theme.color("surface0"));
    }
    frame.render_widget(
        Paragraph::new("下载目录喵~").style(Style::default().fg(theme.color("subtext1"))),
        Rect::new(rect.x + 1, rect.y, 12, 1),
    );
    if app.editing_download_dir {
        let input_width = rect.width.saturating_sub(14);
        let (visible, cursor_column) = visible_input(&app.download_input, true, input_width);
        frame.render_widget(
            Paragraph::new(visible).style(Style::default().fg(theme.color("text"))),
            Rect::new(rect.x + 13, rect.y, input_width, 1),
        );
        let x = rect
            .x
            .saturating_add(13)
            .saturating_add(cursor_column.min(input_width.saturating_sub(1)));
        frame.set_cursor_position(Position::new(x, rect.y));
        return;
    }
    let path = format::truncate(
        &app.download_path_label(),
        rect.width.saturating_sub(22) as usize,
    );
    frame.render_widget(
        Paragraph::new(path).style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(rect.x + 13, rect.y, rect.width.saturating_sub(23), 1),
    );
    let edit_action = Action::StartDownloadDirEdit;
    let edit_width = 8;
    let edit_rect = Rect::new(
        rect.x + rect.width.saturating_sub(edit_width),
        rect.y,
        edit_width,
        1,
    );
    draw_button(
        frame,
        app,
        theme,
        hits,
        edit_rect,
        "编辑喵~",
        edit_action,
        false,
    );
    register(hits, rect, Action::SetSettingsRow(3));
}

fn draw_theme_card(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    flavor: &str,
    hits: &mut Vec<HitBox>,
) {
    let selected = app.theme.flavor == flavor;
    let action = Action::SetTheme(flavor.to_string());
    let hovered = app.hover.as_ref() == Some(&action);
    let preview = Theme::load(&app.paths, flavor);
    let bg = if selected {
        theme.color("surface0")
    } else {
        theme.color("mantle")
    };
    fill(frame, area, bg);
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if selected {
                theme.color("blue")
            } else if hovered {
                theme.color("surface2")
            } else {
                theme.color("surface1")
            })),
        area,
    );
    let label = preview.display_name().to_string();
    frame.render_widget(
        Paragraph::new(label).style(
            Style::default()
                .fg(if selected {
                    theme.color("blue")
                } else {
                    theme.color("text")
                })
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
    );
    let colors = [
        "base", "surface1", "blue", "mauve", "green", "peach", "red", "text",
    ];
    let mut swatches = Vec::new();
    for name in colors {
        swatches.push(Span::styled("  ", Style::default().bg(preview.color(name))));
    }
    frame.render_widget(
        Paragraph::new(Line::from(swatches)),
        Rect::new(area.x + 2, area.y + 2, area.width.saturating_sub(4), 1),
    );
    register(hits, area, action);
}

fn draw_proxy_row(frame: &mut Frame, app: &App, theme: &Theme, hits: &mut Vec<HitBox>, rect: Rect) {
    if app.settings_row == 4 {
        fill(frame, rect, theme.color("surface0"));
    }
    frame.render_widget(
        Paragraph::new("API 代理喵~").style(Style::default().fg(theme.color("subtext1"))),
        Rect::new(rect.x + 1, rect.y, 12, 1),
    );
    if app.editing_proxy {
        let input_width = rect.width.saturating_sub(14);
        let (visible, cursor_column) = visible_input(&app.proxy_input, true, input_width);
        frame.render_widget(
            Paragraph::new(visible).style(Style::default().fg(theme.color("text"))),
            Rect::new(rect.x + 13, rect.y, input_width, 1),
        );
        let x = rect
            .x
            .saturating_add(13)
            .saturating_add(cursor_column.min(input_width.saturating_sub(1)));
        frame.set_cursor_position(Position::new(x, rect.y));
        return;
    }
    frame.render_widget(
        Paragraph::new(format::truncate(
            app.client.proxy(),
            rect.width.saturating_sub(23) as usize,
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(rect.x + 13, rect.y, rect.width.saturating_sub(23), 1),
    );
    let edit_rect = Rect::new(rect.x + rect.width.saturating_sub(9), rect.y, 8, 1);
    draw_button(
        frame,
        app,
        theme,
        hits,
        edit_rect,
        "编辑喵~",
        Action::StartProxyEdit,
        false,
    );
    register(hits, rect, Action::SetSettingsRow(4));
}

#[allow(clippy::too_many_arguments)]
fn draw_ttl_row(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    area: Rect,
    label: &str,
    kind: TtlKind,
    row_index: usize,
) {
    register(hits, area, Action::SetSettingsRow(row_index));
    if app.settings_row == row_index {
        fill(frame, area, theme.color("surface0"));
    }
    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(theme.color("subtext1"))),
        Rect::new(area.x + 1, area.y, 16, 1),
    );
    let value = match kind {
        TtlKind::Repositories => app.config.repos_ttl_secs,
        TtlKind::Detail => app.config.detail_ttl_secs,
        TtlKind::Search => app.config.search_ttl_secs,
    };
    frame.render_widget(
        Paragraph::new(format::ttl_label(value)).style(Style::default().fg(theme.color("text"))),
        Rect::new(area.x + 17, area.y, 12, 1),
    );
    let minus = Action::Ttl(kind, -1);
    let plus = Action::Ttl(kind, 1);
    let minus_rect = Rect::new(area.x + area.width.saturating_sub(17), area.y, 5, 1);
    let plus_rect = Rect::new(area.x + area.width.saturating_sub(9), area.y, 5, 1);
    for (rect, label, action) in [(minus_rect, "-", minus), (plus_rect, "+", plus)] {
        let hovered = app.hover.as_ref() == Some(&action);
        fill(
            frame,
            rect,
            if hovered {
                theme.color("surface1")
            } else {
                theme.color("mantle")
            },
        );
        frame.render_widget(
            Paragraph::new(label)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.color("text"))),
            rect,
        );
        register(hits, rect, action);
    }
}

fn draw_commit_limit_row(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    area: Rect,
) {
    if app.settings_row == 8 {
        fill(frame, area, theme.color("surface0"));
    }
    frame.render_widget(
        Paragraph::new("提交条数喵~").style(Style::default().fg(theme.color("subtext1"))),
        Rect::new(area.x + 1, area.y, 16, 1),
    );
    frame.render_widget(
        Paragraph::new(format!("{} 条", app.config.commit_limit))
            .style(Style::default().fg(theme.color("text"))),
        Rect::new(area.x + 17, area.y, 12, 1),
    );
    for (x, label, action) in [
        (area.width.saturating_sub(17), "-", Action::CommitLimit(-1)),
        (area.width.saturating_sub(9), "+", Action::CommitLimit(1)),
    ] {
        let rect = Rect::new(area.x + x, area.y, 5, 1);
        let hovered = app.hover.as_ref() == Some(&action);
        fill(
            frame,
            rect,
            if hovered {
                theme.color("surface1")
            } else {
                theme.color("mantle")
            },
        );
        frame.render_widget(
            Paragraph::new(label)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.color("text"))),
            rect,
        );
        register(hits, rect, action);
    }
    register(hits, area, Action::SetSettingsRow(8));
}

fn draw_default_page_row(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    area: Rect,
) {
    if app.settings_row == 9 {
        fill(frame, area, theme.color("surface0"));
    }
    let page = Page::from_default_name(&app.config.default_page);
    frame.render_widget(
        Paragraph::new("默认页喵~").style(Style::default().fg(theme.color("subtext1"))),
        Rect::new(area.x + 1, area.y, 16, 1),
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", icons::HOME),
                Style::default().fg(theme.color("pink")),
            ),
            Span::styled(
                page.default_label(),
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
        ])),
        Rect::new(area.x + 17, area.y, 18, 1),
    );
    frame.render_widget(
        Paragraph::new("h/l 换喵~")
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x, area.y, area.width.saturating_sub(1), 1),
    );
    register(hits, area, Action::SetSettingsRow(9));
}

fn render_scrolled(
    frame: &mut Frame,
    area: Rect,
    lines: Vec<Line>,
    actions: Vec<(usize, Action)>,
    scroll: u16,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
) {
    if area.height == 0 {
        return;
    }
    frame.render_widget(Paragraph::new(Text::from(lines)).scroll((scroll, 0)), area);
    let scroll = scroll as usize;
    for (line_index, action) in actions {
        if line_index < scroll || line_index >= scroll + area.height as usize {
            continue;
        }
        let y = area.y + (line_index - scroll) as u16;
        let rect = Rect::new(area.x, y, area.width, 1);
        let hovered = false;
        let _ = hovered;
        register(hits, rect, action);
    }
    let _ = theme;
}

#[allow(clippy::too_many_arguments)]
fn draw_button(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    hits: &mut Vec<HitBox>,
    rect: Rect,
    label: &str,
    action: Action,
    primary: bool,
) {
    let hovered = app.hover.as_ref() == Some(&action);
    let background = if primary {
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
    let foreground = if primary {
        theme.color("base")
    } else if hovered {
        theme.color("text")
    } else {
        theme.color("subtext1")
    };
    fill(frame, rect, background);
    frame.render_widget(
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(Style::default().fg(foreground).add_modifier(Modifier::BOLD)),
        rect,
    );
    register(hits, rect, action);
}

fn draw_center_message(frame: &mut Frame, theme: &Theme, area: Rect, title: &str, detail: &str) {
    let mut lines = vec![
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
    if area.height < 6 {
        lines.truncate(1);
    }
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

fn draw_repo_languages_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let width = area.width.saturating_sub(12).min(64);
    let height = area.height.saturating_sub(8).clamp(10, 24);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    app.layout.overlay = rect;
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
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
    frame.render_widget(
        Paragraph::new(icons::label(icons::CODE, "语言统计喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width.saturating_sub(10), 1),
    );
    let close = Rect::new(inner.x + inner.width.saturating_sub(9), inner.y, 9, 1);
    draw_button(
        frame,
        app,
        theme,
        hits,
        close,
        "关掉喵~",
        Action::CloseOverlay,
        false,
    );
    let total: u64 = app
        .repo_languages
        .iter()
        .map(|language| language.bytes)
        .sum();
    let subtitle = if app.repo_languages_loading.is_some() {
        format!(
            "{} 正在读语言数据喵~...",
            format::spinner(app.started_at.elapsed().as_millis() as u64 / 120)
        )
    } else {
        format!(
            "{} 种语言 · {} 喵~",
            app.repo_languages.len(),
            format::format_size(total)
        )
    };
    frame.render_widget(
        Paragraph::new(format::truncate(&subtitle, inner.width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let list = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(4),
    );
    app.repo_language_viewport = list.height as usize;
    if app.repo_languages.is_empty() {
        draw_center_message(
            frame,
            theme,
            list,
            if app.repo_languages_loading.is_some() {
                "语言还没到喵~"
            } else {
                "没有语言数据喵~"
            },
            if app.repo_languages_loading.is_some() {
                "等一下下，人家正在问 GitHub 喵~"
            } else {
                "按 r 再读一次嘛喵~"
            },
        );
    } else {
        let max_offset = app
            .repo_languages
            .len()
            .saturating_sub(app.repo_language_viewport.max(1));
        app.repo_language_offset = app.repo_language_offset.min(max_offset);
        for (row, language) in app
            .repo_languages
            .iter()
            .skip(app.repo_language_offset)
            .take(app.repo_language_viewport)
            .enumerate()
        {
            let row_rect = Rect::new(list.x, list.y + row as u16, list.width, 1);
            let percent = if total == 0 {
                0.0
            } else {
                language.bytes as f64 * 100.0 / total as f64
            };
            let value = format!("{percent:>5.1}% · {}", format::format_size(language.bytes));
            draw_metric_bar(
                frame,
                theme,
                row_rect,
                &language.name,
                percent as f32 / 100.0,
                &value,
                language_color(theme, &language.name),
                icons::CODE,
            );
        }
    }
    frame.render_widget(
        Paragraph::new("j/k 滚动   r 刷新   Esc 关掉喵~")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            1,
        ),
    );
}

fn draw_new_issue_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let width = area.width.saturating_sub(8).min(78);
    let height = area.height.saturating_sub(6).min(20);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    app.layout.overlay = rect;
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
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
    frame.render_widget(
        Paragraph::new(icons::label(icons::COMMENT, "新建议题喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width.saturating_sub(24), 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(&app.issue_repo_label(), 30))
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.color("blue"))),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new("Tab 换栏   Ctrl+S 提交   Esc 关闭喵~")
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let title_y = inner.y + 3;
    let title_hover = app.hover.as_ref() == Some(&Action::FocusNewIssue(false));
    if !app.new_issue_body_focus || title_hover {
        fill(
            frame,
            Rect::new(inner.x, title_y, inner.width, 1),
            if app.new_issue_body_focus {
                theme.color("surface1")
            } else {
                theme.color("surface0")
            },
        );
    }
    frame.render_widget(
        Paragraph::new("标题喵~").style(Style::default().fg(if app.new_issue_body_focus {
            theme.color("overlay0")
        } else {
            theme.color("blue")
        })),
        Rect::new(inner.x + 1, title_y, 8, 1),
    );
    let value_width = inner.width.saturating_sub(10);
    let (visible, cursor) = if app.new_issue_title.is_empty() {
        (
            format::truncate("给议题起个标题喵~", value_width.saturating_sub(1) as usize),
            0,
        )
    } else {
        format::visible_text(
            &app.new_issue_title.value,
            app.new_issue_title.cursor,
            value_width as usize,
        )
    };
    frame.render_widget(
        Paragraph::new(visible).style(Style::default().fg(if app.new_issue_title.is_empty() {
            theme.color("overlay0")
        } else {
            theme.color("text")
        })),
        Rect::new(inner.x + 9, title_y, value_width, 1),
    );
    if !app.new_issue_body_focus && !app.new_issue_saving {
        frame.set_cursor_position(ratatui::layout::Position::new(
            (inner.x + 9 + cursor).min(inner.x + 8 + value_width),
            title_y,
        ));
    }
    register(
        hits,
        Rect::new(inner.x, title_y, inner.width, 1),
        Action::FocusNewIssue(false),
    );

    let body_label_y = title_y + 2;
    let buttons_y = inner.y + inner.height.saturating_sub(1);
    let error_y = buttons_y.saturating_sub(1);
    frame.render_widget(
        Paragraph::new("正文喵~（可留空）").style(Style::default().fg(
            if app.new_issue_body_focus {
                theme.color("blue")
            } else {
                theme.color("overlay0")
            },
        )),
        Rect::new(inner.x, body_label_y, inner.width, 1),
    );
    let body_outer = Rect::new(
        inner.x,
        body_label_y + 1,
        inner.width,
        error_y.saturating_sub(body_label_y + 1),
    );
    let body_focused = app.new_issue_body_focus;
    let body_hover = app.hover.as_ref() == Some(&Action::FocusNewIssue(true));
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if body_focused {
            theme.color("blue")
        } else if body_hover {
            theme.color("surface2")
        } else {
            theme.color("surface1")
        }));
    let body_inner = block.inner(body_outer);
    frame.render_widget(block, body_outer);
    let width = body_inner.width.max(1) as usize;
    let height = body_inner.height as usize;
    let (all_lines, cursor_row, cursor_column) =
        format::multiline_layout(&app.new_issue_body.value, app.new_issue_body.cursor, width);
    let start = if body_focused {
        cursor_row.saturating_sub(height.saturating_sub(1))
    } else {
        0
    };
    let lines: Vec<Line> = if app.new_issue_body.is_empty() {
        vec![Line::from(Span::styled(
            "写点正文喵~",
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
    frame.render_widget(Paragraph::new(lines), body_inner);
    if body_focused && !app.new_issue_saving && height > 0 {
        let row = cursor_row.saturating_sub(start).min(height - 1);
        let column = cursor_column.min(width.saturating_sub(1));
        frame.set_cursor_position(ratatui::layout::Position::new(
            body_inner.x + column as u16,
            body_inner.y + row as u16,
        ));
    }
    register(hits, body_outer, Action::FocusNewIssue(true));

    if let Some(error) = app.new_issue_error.as_ref() {
        frame.render_widget(
            Paragraph::new(format::truncate(error, inner.width as usize))
                .style(Style::default().fg(theme.color("red"))),
            Rect::new(inner.x, error_y, inner.width, 1),
        );
    }
    let cancel = icons::label(icons::BACK, "取消喵~");
    let cancel_width = format::text_width(&cancel) as u16 + 4;
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, buttons_y, cancel_width, 1),
        &cancel,
        Action::CloseOverlay,
        false,
    );
    let submit = if app.new_issue_saving {
        format!(
            "{} 提交中喵~",
            format::spinner(app.started_at.elapsed().as_millis() as u64 / 120)
        )
    } else {
        icons::label(icons::COMMENT, "提交议题喵~")
    };
    let submit_width = format::text_width(&submit) as u16 + 6;
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(
            inner.x + inner.width.saturating_sub(submit_width),
            buttons_y,
            submit_width,
            1,
        ),
        &submit,
        Action::SubmitNewIssue,
        true,
    );
}

fn draw_repo_edit_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let Some(edit) = app.repo_edit.clone() else {
        return;
    };
    let width = area.width.saturating_sub(12).min(72);
    let height = area.height.saturating_sub(8).min(22);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    app.layout.overlay = rect;
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
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
    frame.render_widget(
        Paragraph::new(icons::label(icons::COGS, "改改自己的仓库喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width.saturating_sub(12), 1),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &format!("{} 只会改你自己的仓库喵~", edit.full_name),
            inner.width as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let mut y = inner.y + 3;
    for (row, label) in [
        "名称",
        "简介",
        "主页",
        "主题",
        "可见性",
        "归档",
        "议题",
        "讨论",
        "Wiki",
    ]
    .iter()
    .enumerate()
    {
        if y >= inner.y + inner.height - 3 {
            break;
        }
        let active = edit.row == row;
        let editing = active && edit.editing;
        if active {
            fill(
                frame,
                Rect::new(inner.x, y, inner.width, 1),
                theme.color("surface0"),
            );
        }
        frame.render_widget(
            Paragraph::new(format!("{label}喵~")).style(if active {
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.color("subtext0"))
            }),
            Rect::new(inner.x + 1, y, 11, 1),
        );
        let value_x = inner.x + 12;
        let value_width = inner.width.saturating_sub(12);
        let (value, cursor) = match row {
            0 => visible_input(&edit.name, editing, value_width),
            1 => visible_input(&edit.description, editing, value_width),
            2 => visible_input(&edit.homepage, editing, value_width),
            3 => visible_input(&edit.topics, editing, value_width),
            4 => (
                if edit.private {
                    "私有 · 只有自己能看到喵~".to_string()
                } else {
                    "公开 · 大家都看得到喵~".to_string()
                },
                0,
            ),
            5 => (
                if edit.archived {
                    "已归档 · 只读状态喵~".to_string()
                } else {
                    "正常 · 还能继续折腾喵~".to_string()
                },
                0,
            ),
            6 => (
                format!(
                    "Issues {}",
                    if edit.has_issues {
                        "开启喵~"
                    } else {
                        "关闭喵~"
                    }
                ),
                0,
            ),
            7 => (
                format!(
                    "Discussions {}",
                    if edit.has_discussions {
                        "开启喵~"
                    } else {
                        "关闭喵~"
                    }
                ),
                0,
            ),
            _ => (
                format!(
                    "Wiki {}",
                    if edit.has_wiki {
                        "开启喵~"
                    } else {
                        "关闭喵~"
                    }
                ),
                0,
            ),
        };
        let empty = match row {
            0 => edit.name.is_empty(),
            1 => edit.description.is_empty(),
            2 => edit.homepage.is_empty(),
            3 => edit.topics.is_empty(),
            _ => false,
        };
        let placeholder = match row {
            0 => "仓库名不能空喵~",
            1 => "还没写简介喵~",
            2 => "没有主页喵~",
            3 => "还没有主题喵~",
            _ => "",
        };
        let shown = if empty {
            placeholder.to_string()
        } else {
            value
        };
        frame.render_widget(
            Paragraph::new(format::truncate(&shown, value_width as usize)).style(if empty {
                Style::default().fg(theme.color("overlay0"))
            } else {
                Style::default().fg(theme.color("text"))
            }),
            Rect::new(value_x, y, value_width, 1),
        );
        if editing {
            let cursor_x = if empty {
                value_x
            } else {
                value_x.saturating_add(cursor.min(value_width.saturating_sub(1)))
            };
            frame.set_cursor_position(Position::new(cursor_x, y));
        }
        register(
            hits,
            Rect::new(inner.x, y, inner.width, 1),
            Action::EditRepoRow(row),
        );
        y += 1;
    }

    if let Some(error) = edit.error.as_deref() {
        frame.render_widget(
            Paragraph::new(format::truncate(error, inner.width as usize))
                .style(Style::default().fg(theme.color("red"))),
            Rect::new(inner.x, y + 1, inner.width, 1),
        );
    }

    let actions_y = inner.y + inner.height.saturating_sub(2);
    let save_label = if edit.saving {
        format!("{} 保存中喵~", app.spinner())
    } else {
        icons::label(icons::SAVE, "保存喵~")
    };
    let save_width = format::text_width(&save_label) as u16 + 4;
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, actions_y, save_width, 1),
        &save_label,
        Action::SaveRepoEdit,
        true,
    );
    draw_button(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x + save_width + 2, actions_y, 12, 1),
        "取消喵~",
        Action::CloseOverlay,
        false,
    );
    if actions_y > inner.y + 2 {
        let hint = if edit.editing {
            "输入中：Enter 完成并下一项   Esc 结束编辑   Ctrl+S 保存喵~"
        } else {
            "j/k 选   Enter 编辑/切换   Ctrl+S 保存   Esc 关掉喵~"
        };
        frame.render_widget(
            Paragraph::new(hint)
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x, actions_y - 2, inner.width, 1),
        );
    }
}

fn draw_languages_overlay(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let choices = app.language_choices();
    let width = area.width.saturating_sub(12).min(58);
    let height = area.height.saturating_sub(8).clamp(10, 24);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    app.layout.overlay = rect;
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
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
    frame.render_widget(
        Paragraph::new(icons::label(icons::CODE, "挑一个语言喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width.saturating_sub(10), 1),
    );
    let close = Rect::new(inner.x + inner.width.saturating_sub(9), inner.y, 9, 1);
    draw_button(
        frame,
        app,
        theme,
        hits,
        close,
        "关掉喵~",
        Action::CloseOverlay,
        false,
    );
    frame.render_widget(
        Paragraph::new(format!("一共 {} 项，人家都给你列出来啦喵~", choices.len()))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let list = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(4),
    );
    app.language_viewport = list.height as usize;
    let max_offset = choices.len().saturating_sub(app.language_viewport.max(1));
    app.language_offset = app.language_offset.min(max_offset);
    for (row, (value, label, count)) in choices
        .iter()
        .skip(app.language_offset)
        .take(app.language_viewport)
        .enumerate()
    {
        let index = app.language_offset + row;
        let rect = Rect::new(list.x, list.y + row as u16, list.width, 1);
        let active = value.as_deref() == app.language_filter.as_deref();
        let selected = index == app.language_selected;
        if active || selected {
            fill(
                frame,
                rect,
                if active {
                    theme.color("surface1")
                } else {
                    theme.color("surface0")
                },
            );
        }
        if active {
            fill(frame, Rect::new(rect.x, rect.y, 1, 1), theme.color("blue"));
        }
        frame.render_widget(
            Paragraph::new(format!(
                "{} {}",
                if active { icons::CHECK } else { icons::SQUARE },
                label
            ))
            .style(if active {
                Style::default()
                    .fg(theme.color("blue"))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.color("subtext1"))
            }),
            Rect::new(rect.x + 1, rect.y, rect.width.saturating_sub(10), 1),
        );
        frame.render_widget(
            Paragraph::new(count.to_string())
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(rect.x, rect.y, rect.width.saturating_sub(2), 1),
        );
        register(hits, rect, Action::Language(value.clone()));
    }
    frame.render_widget(
        Paragraph::new("j/k 挑选   Enter 决定   Esc 关掉喵~")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            1,
        ),
    );
}

fn draw_palette(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    _hits: &mut Vec<HitBox>,
) {
    let width = area.width.saturating_sub(12).clamp(36, 70);
    let height = area.height.saturating_sub(6).clamp(9, 19);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 3,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.color("mauve"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    let items = app.palette_filtered();
    app.palette_selected = app.palette_selected.min(items.len().saturating_sub(1));
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", icons::COMMAND),
                Style::default().fg(theme.color("mauve")),
            ),
            Span::styled(
                "命令面板喵~",
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("   共 {} 条", items.len()),
                Style::default().fg(theme.color("overlay0")),
            ),
        ])),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    let field = Rect::new(inner.x, inner.y + 2, inner.width, 1);
    fill(frame, field, theme.color("surface0"));
    let (visible, cursor) = visible_input(&app.palette_input, true, inner.width.saturating_sub(3));
    let text = if app.palette_input.is_empty() {
        Span::styled(
            "输入命令或页面名喵~",
            Style::default().fg(theme.color("overlay0")),
        )
    } else {
        Span::styled(visible, Style::default().fg(theme.color("text")))
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", icons::SEARCH),
                Style::default().fg(theme.color("mauve")),
            ),
            text,
        ])),
        field,
    );
    frame.set_cursor_position(Position::new(
        field.x + 2 + cursor.min(inner.width.saturating_sub(3)),
        field.y,
    ));

    let list = Rect::new(
        inner.x,
        inner.y + 4,
        inner.width,
        inner.height.saturating_sub(5),
    );
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("没有匹配的命令喵~换个词试试嘛")
                .style(Style::default().fg(theme.color("overlay0"))),
            list,
        );
    } else {
        let capacity = list.height as usize;
        let offset = app
            .palette_selected
            .saturating_sub(capacity.saturating_sub(1));
        for (slot, item) in items.iter().enumerate().skip(offset).take(capacity) {
            let index = slot;
            let y = list.y + (index - offset) as u16;
            let row = Rect::new(list.x, y, list.width, 1);
            let selected = index == app.palette_selected;
            if selected {
                fill(frame, row, theme.color("surface1"));
                fill(frame, Rect::new(row.x, row.y, 1, 1), theme.color("mauve"));
            }
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        format!(" {} ", icons::SPARKLE),
                        Style::default().fg(if selected {
                            theme.color("mauve")
                        } else {
                            theme.color("overlay0")
                        }),
                    ),
                    Span::styled(
                        item.label.clone(),
                        Style::default()
                            .fg(if selected {
                                theme.color("text")
                            } else {
                                theme.color("subtext1")
                            })
                            .add_modifier(if selected {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                    ),
                ])),
                row,
            );
            if row.width > 34 {
                frame.render_widget(
                    Paragraph::new(format::truncate(
                        &item.hint,
                        row.width.saturating_sub(30) as usize,
                    ))
                    .alignment(Alignment::Right)
                    .style(Style::default().fg(theme.color("overlay0"))),
                    row,
                );
            }
        }
    }
    frame.render_widget(
        Paragraph::new("↑↓ 挑   Enter 执行   Esc 关闭喵~")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            1,
        ),
    );
    app.layout.overlay = rect;
}

fn draw_theme_studio(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    let width = area.width.saturating_sub(16).clamp(36, 62);
    let height = (app.themes.len() as u16 + 8)
        .min(area.height.saturating_sub(4))
        .max(9);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 3,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    fill(frame, rect, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.color("sapphire"))),
        rect,
    );
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", icons::THEME),
                Style::default().fg(theme.color("sapphire")),
            ),
            Span::styled(
                "主题工作室喵~",
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "   Enter 应用喵~",
                Style::default().fg(theme.color("overlay0")),
            ),
        ])),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new("挑一套喜欢的配色，改完记得保存设置喵~")
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let list = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(4),
    );
    let capacity = list.height as usize;
    let offset = app
        .theme_selected
        .saturating_sub(capacity.saturating_sub(1));
    for (slot, name) in app.themes.iter().enumerate().skip(offset).take(capacity) {
        let y = list.y + (slot - offset) as u16;
        let row = Rect::new(list.x, y, list.width, 1);
        let selected = slot == app.theme_selected;
        let active = *name == app.theme.flavor;
        let preview = Theme::load(&app.paths, name);
        if selected {
            fill(frame, row, theme.color("surface0"));
        }
        if selected || active {
            fill(
                frame,
                Rect::new(row.x, row.y, 1, 1),
                if active {
                    theme.color("green")
                } else {
                    theme.color("sapphire")
                },
            );
        }
        let mark = if active { icons::CHECK } else { " " };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!(" {mark} {} ", icons::PALETTE),
                    Style::default().fg(if active {
                        theme.color("green")
                    } else {
                        theme.color("overlay0")
                    }),
                ),
                Span::styled(
                    format::truncate(preview.display_name(), 18),
                    Style::default()
                        .fg(if selected {
                            theme.color("text")
                        } else {
                            theme.color("subtext1")
                        })
                        .add_modifier(if selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                Span::styled(
                    format!("  {name}"),
                    Style::default().fg(theme.color("overlay0")),
                ),
            ])),
            row,
        );
        let swatch_width = 6_u16;
        if row.width > swatch_width + 12 {
            let mut swatches = Vec::new();
            for key in ["blue", "mauve", "green", "peach", "text"] {
                swatches.push(Span::styled("■ ", Style::default().fg(preview.color(key))));
            }
            swatches.push(Span::styled(
                if active { "已启用" } else { "" },
                Style::default().fg(theme.color("green")),
            ));
            frame.render_widget(
                Paragraph::new(Line::from(swatches)).alignment(Alignment::Right),
                Rect::new(
                    row.x + row.width.saturating_sub(18),
                    row.y,
                    row.width.saturating_sub(row.width.saturating_sub(18)),
                    1,
                ),
            );
        }
        register(hits, row, Action::SetTheme(name.clone()));
    }
    frame.render_widget(
        Paragraph::new("↑↓ 挑   Enter 应用   Esc 关闭喵~")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            1,
        ),
    );
    app.layout.overlay = rect;
}

fn draw_help(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, hits: &mut Vec<HitBox>) {
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
            "换页喵~",
            "0 首页  1 仓库  2 收藏  3 搜索  4 账号  5 通知  6 热门  9 洞察  Tab 切页喵~",
        ),
        (
            "命令喵~",
            "Ctrl+K 或 : 打开命令面板，输入页面或动作名就能跳过去喵~",
        ),
        (
            "首页喵~",
            "P 把当前仓库钉到首页，j/k 挑置顶，Enter 打开，E 随机探索喵~",
        ),
        (
            "美化喵~",
            "T 打开主题工作室，挑 Catppuccin 配色，Enter 立即换色喵~",
        ),
        (
            "挑挑喵~",
            "j / k 上下挑  [ ] 翻页  g / G 首尾页  PgUp/PgDn 翻页喵~",
        ),
        (
            "看仓库喵~",
            "Enter 进仓库页  o 去浏览器  y 复制  Y Fork  D 删除喵~",
        ),
        (
            "改自己喵~",
            "自己的仓库按 e 改名称、简介、主页、主题、可见性、归档和功能开关喵~",
        ),
        (
            "找找喵~",
            "s 输入关键词  / 在结果里筛选  l 挑语言  Enter 打开喵~",
        ),
        ("热门喵~", "6 看热门仓库，p 换今日/本周/本月，/ 筛选喵~"),
        (
            "看说明喵~",
            "仓库页按 F 全屏看 README，L 看语言统计，TAB 切面板，j/k 滚动喵~",
        ),
        (
            "提交喵~",
            "仓库页按 C 看提交和改动文件，j/k 挑，Enter 去 GitHub，b 换分支喵~",
        ),
        (
            "分支喵~",
            "仓库页或提交页按 b 选分支，树和 README 会跟着换喵~",
        ),
        ("账号喵~", "4 看账号信息，还能查 GitHub API 额度喵~"),
        (
            "工作流喵~",
            "仓库页按 a 看 Actions，R/x 跑工作流，Tab 填分支和 inputs 喵~",
        ),
        (
            "议题喵~",
            "仓库页按 i 看议题，v 换类型，s 换状态，n 新建一条喵~",
        ),
        ("发行版喵~", "仓库页点发行版按钮进详情，再挑文件下载喵~"),
        ("通知喵~", "5 看通知，m 已读，M 全部已读，u 切未读/全部喵~"),
        ("缓存喵~", "按 c 分类清理缓存，o 打开缓存目录喵~"),
        ("设置喵~", "退出后敲 gxt settings，用专门的设置 TUI 调喵~"),
        ("鼠标喵~", "点一下挑，点两下进仓库页，点作者名看主页喵~"),
        ("回去喵~", "Esc 回列表  ? 关掉说明  q 退出人家喵~"),
    ];
    let mut y = inner.y;
    frame.render_widget(
        Paragraph::new(icons::label(icons::INFO, "人家能帮你做什么喵~")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 2;
    for (label, text) in lines {
        if y >= inner.y + inner.height {
            break;
        }
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{label:<6}"),
                    Style::default().fg(theme.color("blue")),
                ),
                Span::styled(text, Style::default().fg(theme.color("subtext0"))),
            ])),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y += 1;
    }
    let close = Rect::new(inner.x, inner.y + inner.height.saturating_sub(1), 10, 1);
    draw_button(
        frame,
        app,
        theme,
        hits,
        close,
        "关掉喵~",
        Action::CloseOverlay,
        false,
    );
    app.layout.overlay = rect;
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
        Line::from(Span::styled(
            "按 Ctrl+C 可以退出喵~",
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

fn colorize_icons(buffer: &mut ratatui::buffer::Buffer, theme: &Theme) {
    let area = *buffer.area();
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            let is_icon = {
                let Some(cell) = buffer.cell((x, y)) else {
                    continue;
                };
                let mut chars = cell.symbol().chars();
                matches!(
                    (chars.next(), chars.next()),
                    (Some(ch), None) if (0xE000..=0xF8FF).contains(&(ch as u32))
                )
            };
            if !is_icon {
                continue;
            }
            let symbol = buffer
                .cell((x, y))
                .map(|cell| cell.symbol().to_string())
                .unwrap_or_default();
            if let Some(cell) = buffer.cell_mut((x, y)) {
                cell.set_fg(icon_tint(&symbol, theme));
            }
        }
    }
}

fn icon_tint(symbol: &str, theme: &Theme) -> Color {
    if symbol == icons::STAR || symbol == icons::STAR_EMPTY || symbol == icons::SPARKLE {
        theme.color("yellow")
    } else if symbol == icons::FLAME || symbol == icons::TRASH || symbol == icons::WARNING {
        theme.color("red")
    } else if symbol == icons::REPO
        || symbol == icons::FOLDER_OPEN
        || symbol == icons::DOWNLOAD_DIR
        || symbol == icons::INFO
        || symbol == icons::GITHUB
    {
        theme.color("blue")
    } else if symbol == icons::USER || symbol == icons::PLAY {
        theme.color("green")
    } else if symbol == icons::BELL
        || symbol == icons::TAG
        || symbol == icons::DOWNLOAD
        || symbol == icons::ARCHIVE
        || symbol == icons::RANDOM
    {
        theme.color("peach")
    } else if symbol == icons::SEARCH
        || symbol == icons::EXTERNAL
        || symbol == icons::EYE
        || symbol == icons::EXPAND
        || symbol == icons::CODE
        || symbol == icons::CODE_FILE
    {
        theme.color("sky")
    } else if symbol == icons::LOCK || symbol == icons::PIN || symbol == icons::CHART {
        theme.color("mauve")
    } else if symbol == icons::GLOBE || symbol == icons::NETWORK {
        theme.color("teal")
    } else if symbol == icons::ISSUE
        || symbol == icons::CHECK
        || symbol == icons::CHECK_CIRCLE
        || symbol == icons::CHECK_SQUARE
        || symbol == icons::SAVE
        || symbol == icons::REFRESH
        || symbol == icons::TERMINAL
        || symbol == icons::COMMAND
    {
        theme.color("green")
    } else if symbol == icons::CLOCK || symbol == icons::COMMIT {
        theme.color("yellow")
    } else if symbol == icons::COPY
        || symbol == icons::LICENSE
        || symbol == icons::COMMENT
        || symbol == icons::BOOK
        || symbol == icons::BRANCH
        || symbol == icons::PULL
        || symbol == icons::COGS
    {
        theme.color("lavender")
    } else if symbol == icons::DATABASE {
        theme.color("peach")
    } else if symbol == icons::PALETTE
        || symbol == icons::THEME
        || symbol == icons::IMAGE
        || symbol == icons::HOME
    {
        theme.color("pink")
    } else if symbol == icons::BACK
        || symbol == icons::CHEVRON_LEFT
        || symbol == icons::CHEVRON_RIGHT
    {
        theme.color("subtext0")
    } else if symbol == icons::X_CIRCLE {
        theme.color("red")
    } else if symbol == icons::SQUARE {
        theme.color("overlay1")
    } else {
        let palette = [
            theme.color("blue"),
            theme.color("mauve"),
            theme.color("green"),
            theme.color("peach"),
            theme.color("sky"),
            theme.color("pink"),
            theme.color("yellow"),
            theme.color("teal"),
        ];
        let code = symbol.chars().next().unwrap_or('?') as usize;
        palette[code % palette.len()]
    }
}

fn register_owner_hit(hits: &mut Vec<HitBox>, x: u16, y: u16, width: u16, owner: &str) {
    if owner.is_empty() || width == 0 {
        return;
    }
    let icon_width = format::text_width(&format!("{} ", icons::REPO)) as u16;
    if icon_width >= width {
        return;
    }
    let owner_width = format::text_width(owner) as u16;
    if owner_width == 0 {
        return;
    }
    register(
        hits,
        Rect::new(x + icon_width, y, owner_width.min(width - icon_width), 1),
        Action::OpenProfile(owner.to_string()),
    );
}

fn register(hits: &mut Vec<HitBox>, rect: Rect, action: Action) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    hits.push(HitBox { rect, action });
}

fn fill(frame: &mut Frame, rect: Rect, color: Color) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    frame.render_widget(Block::new().style(Style::default().bg(color)), rect);
}

fn language_color(theme: &Theme, language: &str) -> Color {
    const NAMES: [&str; 10] = [
        "blue", "mauve", "green", "peach", "teal", "yellow", "sapphire", "lavender", "pink", "sky",
    ];
    if language == "未标注喵~" {
        return theme.color("overlay0");
    }
    let hash = language.bytes().fold(0usize, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(byte as usize)
    });
    theme.color(NAMES[hash % NAMES.len()])
}

fn proxy_label(app: &App) -> String {
    app.client
        .proxy()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string()
}

fn visible_input(input: &TextInput, focused: bool, max_width: u16) -> (String, u16) {
    if focused {
        input.visible_window(max_width as usize)
    } else {
        (format::truncate(&input.value, max_width.max(1) as usize), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_nested_markdown_images() {
        let line = "[![GitHub last commit](https://img.example/a)](https://github.com/x)";
        assert_eq!(strip_markdown_inline(line), "");
        assert!(strip_markdown_images("![badge](https://img) text").contains(" text"));
        assert_eq!(
            strip_markdown_inline("**bold** and `code`"),
            "bold and code"
        );
    }

    #[test]
    fn renders_rich_inline_and_code_fences() {
        let paths = crate::config::Paths::for_test("missing".into(), "missing".into());
        let theme = Theme::load(&paths, "mocha");
        let markdown = "**粗体** 和 *斜体* 和 `代码` 和 ~~删掉~~\n\n> [!TIP]\n> 先看这里\n\n```rust\nfn main() { let x = 1; }\n```\n";
        let lines = markdown_lines(markdown, 80, &theme);
        let spans: Vec<&Span> = lines.iter().flat_map(|line| line.spans.iter()).collect();
        let text = |span: &Span| span.content.to_string();
        assert!(spans.iter().any(|span| text(span).contains("粗体")));
        assert!(
            lines
                .iter()
                .flat_map(|line| line.spans.iter())
                .any(|span| span.content.contains("粗体")
                    && span.style.add_modifier.contains(Modifier::BOLD))
        );
        assert!(spans.iter().any(|span| text(span).contains("TIP")));
        assert!(spans.iter().any(|span| text(span).contains("rust")));
        assert!(spans.iter().any(|span| text(span) == "main"));
        assert!(lines.iter().all(|line| line.width() <= 80));
        let tasks = markdown_lines("- [x] 完成\n- [ ] 未完成\n", 60, &theme);
        let task_text: String = tasks
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(task_text.contains("任务 1/2"));
        assert!(task_text.contains('█'));
        let code_span = spans
            .iter()
            .find(|span| span.content.contains("代码"))
            .unwrap();
        assert_eq!(code_span.style.bg, Some(theme.color("surface0")));
    }

    #[test]
    fn keeps_pipe_text_and_groups_blockquotes() {
        let paths = crate::config::Paths::for_test("missing".into(), "missing".into());
        let theme = Theme::load(&paths, "mocha");
        let markdown = "a | b\n继续写\n\n> 第一行\n> 第二行\n\n- [x] 完成\n- [ ] 未完成\n";
        let lines = markdown_lines(markdown, 80, &theme);
        let text: Vec<String> = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect();
        assert!(text.iter().any(|line| line.contains("a | b 继续写")));
        assert!(text.iter().filter(|line| line.starts_with("▎ ")).count() >= 2);
        assert!(text.iter().any(|line| line.contains(icons::CHECK_SQUARE)));
        assert!(text.iter().any(|line| line.contains(icons::SQUARE)));
    }

    #[test]
    fn shows_markdown_image_chips_but_hides_badges() {
        let paths = crate::config::Paths::for_test("missing".into(), "missing".into());
        let theme = Theme::load(&paths, "mocha");
        let markdown = "![预览](./assets/images/preview.png)\n\n[![badge](https://img.shields.io/x)](https://example.com)\n\n<img src=\"./assets/images/preview.png\" alt=\"x\">\n";
        let lines = markdown_lines(markdown, 80, &theme);
        let text = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("图片 预览"));
        assert!(text.contains("图片 ./assets/images/preview.png"));
        assert!(!text.contains("badge"));
    }

    #[test]
    fn renders_markdown_tables_within_width() {
        let paths = crate::config::Paths::for_test("missing".into(), "missing".into());
        let theme = Theme::load(&paths, "mocha");
        let markdown = "# 标题\n\n| 名称 | 说明 |\n| --- | :---: |\n| gxt | 终端里的工作台，支持很多很多很多很多很多很多很多很多很多很多很多功能喵~ |\n\n普通段落。\n";
        for width in [20usize, 40, 80] {
            let lines = markdown_lines(markdown, width, &theme);
            assert!(
                lines
                    .iter()
                    .any(|line| line.spans.iter().any(|span| span.content.contains('╭'))),
                "table should be drawn as a grid"
            );
            for line in lines {
                assert!(line.width() <= width, "line wider than {width}: {:?}", line);
            }
        }
    }

    fn test_repo(owner: &str, name: &str) -> Repo {
        Repo {
            name: name.to_string(),
            full_name: format!("{owner}/{name}"),
            owner: crate::model::Owner {
                login: owner.to_string(),
                ..Default::default()
            },
            description: Some("a test repo".to_string()),
            ..Default::default()
        }
    }

    fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
        let width = buffer.area.width as usize;
        buffer
            .content()
            .chunks(width)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn account_page_renders_profile_and_stats() {
        let dir = std::env::temp_dir().join(format!("gitx-account-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.page = Page::Account;
        app.user = Some(crate::model::User {
            login: "octocat".to_string(),
            name: Some("The Octocat".to_string()),
            public_repos: 2,
            followers: 3,
            following: 4,
            public_gists: 5,
            ..Default::default()
        });
        app.repos = vec![test_repo("octocat", "hello-world")];
        app.starred = vec![test_repo("octocat", "spoon-knife")];
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render account");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("公开Gist"));
        assert!(text.contains("收藏星标"));
        assert!(text.contains("正在关注"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn profile_page_renders_details_and_all_repos() {
        let dir = std::env::temp_dir().join(format!("gitx-profile-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.page = Page::Profile;
        app.profile_login = Some("octocat".to_string());
        app.profile_user = Some(crate::model::User {
            login: "octocat".to_string(),
            name: Some("The Octocat".to_string()),
            bio: Some("cat".to_string()),
            public_repos: 1,
            ..Default::default()
        });
        app.profile_repos = vec![test_repo("octocat", "hello-world")];
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render profile");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("octocat"));
        assert!(text.contains("全部仓库"));
        assert!(text.contains("hello-world"));
        assert!(
            app.hitboxes
                .iter()
                .any(|hit| matches!(&hit.action, Action::OpenProfile(login) if login == "octocat"))
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn repo_page_offers_fork_and_author_profile() {
        let dir = std::env::temp_dir().join(format!("gitx-fork-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.login = Some("me".to_string());
        let repo = test_repo("octocat", "hello-world");
        app.page = Page::Repo;
        app.return_page = Page::Repositories;
        app.page_repo = Some(repo.clone());
        app.detail = Some(repo);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render repo");
        assert!(
            app.hitboxes
                .iter()
                .any(|hit| hit.action == Action::ForkRepo)
        );
        assert!(
            app.hitboxes
                .iter()
                .any(|hit| matches!(&hit.action, Action::OpenProfile(login) if login == "octocat"))
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn author_title_underlines_on_hover() {
        let paths = crate::config::Paths::for_test("missing".into(), "missing".into());
        let theme = Theme::load(&paths, "mocha");
        let repo = test_repo("octocat", "hello-world");
        let line = repo_title_line(&theme, &repo, true, 60);
        assert!(line.spans.iter().any(|span| {
            span.content.contains("octocat")
                && span.style.add_modifier.contains(Modifier::UNDERLINED)
        }));
        let line = repo_title_line(&theme, &repo, false, 60);
        assert!(line.spans.iter().all(|span| {
            !span.content.contains("octocat")
                || !span.style.add_modifier.contains(Modifier::UNDERLINED)
        }));
    }

    #[test]
    fn fork_and_delete_overlays_show_confirm_buttons() {
        let dir = std::env::temp_dir().join(format!("gitx-confirm-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.login = Some("me".to_string());
        let repo = test_repo("octocat", "hello-world");
        app.page = Page::Repo;
        app.return_page = Page::Repositories;
        app.page_repo = Some(repo.clone());
        app.detail = Some(repo.clone());
        app.fork_repo = Some(repo.clone());
        app.overlay = Some(Overlay::ForkRepo);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 32))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render fork overlay");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("Fork仓库"));
        assert!(
            app.hitboxes
                .iter()
                .any(|hit| hit.action == Action::ConfirmFork)
        );

        app.overlay = Some(Overlay::DeleteRepo);
        app.fork_repo = None;
        app.delete_repo = Some(repo);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 32))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render delete overlay");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("删除仓库"));
        assert!(text.contains("不能撤销"));
        assert!(
            app.hitboxes
                .iter()
                .any(|hit| hit.action == Action::ConfirmDeleteRepo)
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn repo_languages_overlay_renders_percentages() {
        let dir = std::env::temp_dir().join(format!("gitx-lang-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        let repo = test_repo("octocat", "hello-world");
        app.page = Page::Repo;
        app.return_page = Page::Repositories;
        app.page_repo = Some(repo.clone());
        app.detail = Some(repo);
        app.overlay = Some(Overlay::RepoLanguages);
        app.repo_languages = vec![
            crate::model::LanguageStat {
                name: "Rust".to_string(),
                bytes: 750,
            },
            crate::model::LanguageStat {
                name: "Shell".to_string(),
                bytes: 250,
            },
        ];
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render languages overlay");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("语言统计"));
        assert!(text.contains("Rust"));
        assert!(text.contains("75.0%"));
        assert!(text.contains("Shell"));
        assert!(
            app.hitboxes
                .iter()
                .any(|hit| hit.action == Action::CloseOverlay)
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn new_issue_overlay_exposes_fields_and_submit() {
        let dir = std::env::temp_dir().join(format!("gitx-issue-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        let repo = test_repo("octocat", "hello-world");
        app.page = Page::Issues;
        app.return_page = Page::Repositories;
        app.issue_repo = Some(repo);
        app.overlay = Some(Overlay::NewIssue);
        app.new_issue_title = TextInput::new("新的喵");
        app.new_issue_body = TextInput::new("正文");
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render new issue overlay");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("新建议题"));
        assert!(text.contains("正文喵"));
        assert!(text.contains("提交议题"));
        assert!(
            app.hitboxes
                .iter()
                .any(|hit| hit.action == Action::SubmitNewIssue)
        );
        assert!(
            app.hitboxes
                .iter()
                .any(|hit| matches!(&hit.action, Action::FocusNewIssue(body) if !body))
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn commit_detail_lists_changed_files() {
        let dir = std::env::temp_dir().join(format!("gitx-commit-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        let repo = test_repo("octocat", "hello-world");
        let sha = "abcdef1234567890".to_string();
        app.page = Page::Commits;
        app.return_page = Page::Repo;
        app.page_repo = Some(repo.clone());
        app.commit_repo = Some(repo);
        app.commits = vec![crate::model::Commit {
            sha: sha.clone(),
            html_url: "https://github.com/octocat/hello-world/commit/abcdef1".to_string(),
            commit: crate::model::CommitInfo {
                message: "fix: 文件喵".to_string(),
                author: crate::model::CommitAuthor {
                    name: "octocat".to_string(),
                    date: "2026-01-01T00:00:00Z".to_string(),
                },
            },
            ..Default::default()
        }];
        app.commit_detail = Some(crate::model::CommitDetail {
            sha: sha.clone(),
            stats: Some(crate::model::CommitStats {
                additions: 12,
                deletions: 3,
                total: 15,
            }),
            files: vec![crate::model::CommitFile {
                filename: "src/main.rs".to_string(),
                status: "modified".to_string(),
                additions: 12,
                deletions: 3,
                changes: 15,
                patch: None,
            }],
            ..Default::default()
        });
        app.commit_detail_for = Some(sha);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render commit detail");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("src/main.rs"));
        assert!(text.contains("+12"));
        assert!(text.contains("-3"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn home_page_renders_stats_pins_and_quick_actions() {
        let dir = std::env::temp_dir().join(format!("gitx-home-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.page = Page::Home;
        app.login = Some("octocat".to_string());
        app.repos = vec![test_repo("octocat", "hello-world")];
        app.starred = vec![test_repo("octocat", "spoon-knife")];
        app.config.pinned_repos = vec!["octocat/hello-world".to_string()];
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render home");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("欢迎回来"));
        assert!(text.contains("hello-world"));
        assert!(text.contains("置顶仓库"));
        assert!(text.contains("快捷入口"));
        assert!(text.contains("语言分布"));
        assert!(
            app.hitboxes
                .iter()
                .any(|hit| matches!(hit.action, Action::OpenPin(0)))
        );
        assert!(
            app.hitboxes
                .iter()
                .any(|hit| hit.action == Action::OpenPalette)
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn insights_page_renders_language_rank_and_quota() {
        let dir = std::env::temp_dir().join(format!("gitx-insights-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.page = Page::Insights;
        let mut rust = test_repo("octocat", "hello-world");
        rust.language = Some("Rust".to_string());
        rust.stargazers_count = 42;
        let mut cat = test_repo("octocat", "spoon-knife");
        cat.language = Some("Lua".to_string());
        app.repos = vec![rust];
        app.starred = vec![cat];
        app.rate_limit = Some(crate::model::RateLimit {
            resources: crate::model::RateLimitResources {
                core: crate::model::RateLimitInfo {
                    limit: 100,
                    used: 10,
                    remaining: 90,
                    reset: 0,
                },
                ..Default::default()
            },
        });
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render insights");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("数据洞察"));
        assert!(text.contains("语言分布"));
        assert!(text.contains("星标排行"));
        assert!(text.contains("API额度"));
        assert!(text.contains("Rust"));
        assert!(text.contains("hello-world"));
        assert!(text.contains("90/100"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn palette_filters_commands_and_theme_studio_lists_colors() {
        let dir = std::env::temp_dir().join(format!("gitx-palette-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.page = Page::Home;
        app.open_palette();
        app.palette_input = crate::app::TextInput::new("洞察");
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render palette");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("命令面板"));
        assert!(text.contains("数据洞察"));
        assert!(!text.contains("缓存管理"));

        app.overlay = None;
        app.open_themes();
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render theme studio");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("主题工作室"));
        assert!(text.contains("Mocha"));
        assert!(text.contains("Latte"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn header_scrolls_tabs_when_narrow() {
        let dir = std::env::temp_dir().join(format!("gitx-nav-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.page = Page::Home;
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(84, 30))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render narrow header");
        assert!(app.hitboxes.iter().any(|hit| hit.action == Action::NavLeft));
        assert!(
            app.hitboxes
                .iter()
                .any(|hit| hit.action == Action::NavRight)
        );
        let text = buffer_text(terminal.backend().buffer());
        let compact = text.split_whitespace().collect::<String>();
        assert!(!compact.contains("帮助喵~"));
        assert!(compact.contains("首页喵~"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn icons_get_theme_colors() {
        let dir = std::env::temp_dir().join(format!("gitx-icon-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        app.page = Page::Home;
        app.login = Some("octocat".to_string());
        let pink = app.theme.color("pink");
        let yellow = app.theme.color("yellow");
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render icons");
        let buffer = terminal.backend().buffer();
        assert!(
            buffer
                .content()
                .iter()
                .any(|cell| cell.symbol() == icons::HOME && cell.fg == pink)
        );
        assert!(
            buffer
                .content()
                .iter()
                .any(|cell| cell.symbol() == icons::STAR && cell.fg == yellow)
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn settings_page_exposes_default_page_row() {
        let dir = std::env::temp_dir().join(format!("gitx-settings-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.page = Page::Settings;
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render settings");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("默认页"));
        assert!(
            app.hitboxes
                .iter()
                .any(|hit| hit.action == Action::SetSettingsRow(9))
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn repository_release_rows_show_pills_and_state_badges() {
        let dir = std::env::temp_dir().join(format!("gitx-release-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        let repo = test_repo("octocat", "hello-world");
        app.page = Page::Repo;
        app.return_page = Page::Repositories;
        app.page_repo = Some(repo.clone());
        app.detail = Some(repo);
        app.repo_pane = crate::app::RepoPane::Releases;
        app.releases = vec![
            crate::model::Release {
                tag_name: "v2.0.0".to_string(),
                name: Some("新版本".to_string()),
                ..Default::default()
            },
            crate::model::Release {
                tag_name: "v1.9.0-rc1".to_string(),
                prerelease: true,
                ..Default::default()
            },
        ];
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render release rows");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("v2.0.0"));
        assert!(text.contains("v1.9.0-rc1"));
        assert!(text.contains("最新"));
        assert!(text.contains("预发布"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn release_detail_draws_tag_pill_and_latest_badge() {
        let dir = std::env::temp_dir().join(format!("gitx-release-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = crate::config::Paths::for_test(dir.join("config"), dir.join("cache"));
        let mut app = App::settings(paths, None);
        app.settings_only = false;
        let repo = test_repo("octocat", "hello-world");
        app.page = Page::ReleaseDetail;
        app.return_page = Page::Repo;
        app.page_repo = Some(repo);
        app.releases = vec![crate::model::Release {
            tag_name: "v1.2.3".to_string(),
            name: Some("第一版".to_string()),
            ..Default::default()
        }];
        app.release_selected = 0;
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
            .expect("test terminal");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("render release");
        let text = buffer_text(terminal.backend().buffer())
            .split_whitespace()
            .collect::<String>();
        assert!(text.contains("v1.2.3"));
        assert!(text.contains("最新"));
        assert!(text.contains("发行说明"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
