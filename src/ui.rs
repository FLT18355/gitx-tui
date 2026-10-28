use crate::app::{Action, App, HitBox, LayoutRects, Overlay, Page, StatusKind, TextInput, TtlKind};
use crate::format;
use crate::icons;
use crate::model::{Repo, RepoSort, SearchSort};
use crate::theme::{FLAVORS, Theme};
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
        Page::Repositories => draw_repositories_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Starred => draw_starred_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Search => draw_search_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Settings => draw_settings_page(frame, app, &theme, chunks[1], &mut hits),
        Page::Repo => draw_repo_page(frame, app, &theme, chunks[1], &mut hits),
    }
    draw_footer(frame, app, &theme, chunks[2], &hits);

    if app.overlay == Some(Overlay::Help) {
        draw_help(frame, app, &theme, area, &mut hits);
    }

    app.hitboxes = hits;
}

fn draw_header(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, hits: &mut Vec<HitBox>) {
    fill(frame, area, theme.color("mantle"));
    let brand = Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1);
    let mut title = vec![
        Span::styled(
            format!("{} ", icons::GITHUB),
            Style::default().fg(theme.color("blue")),
        ),
        Span::styled(
            "gitx-tui",
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if area.width > 96 {
        title.push(Span::styled(
            "  终端里的 GitHub 工作台",
            Style::default().fg(theme.color("overlay0")),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(title)), brand);

    let account = format!(
        "{} 账号 {}   {} {}",
        icons::USER,
        app.auth_label(),
        icons::PALETTE,
        Theme::label(&app.theme.flavor)
    );
    if area.width > 84 {
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

    let active = app
        .page
        .nav_index()
        .unwrap_or_else(|| app.return_page.nav_index().unwrap_or(0));
    let labels = [
        icons::label(icons::REPO, "仓库"),
        icons::label(icons::STAR, "收藏"),
        icons::label(icons::SEARCH, "搜索"),
        icons::label(icons::SETTINGS, "设置"),
    ];
    let pages = [
        Page::Repositories,
        Page::Starred,
        Page::Search,
        Page::Settings,
    ];
    let mut x = area.x + 2;
    for (index, label) in labels.iter().enumerate() {
        let width = format::text_width(label) as u16 + 4;
        let rect = Rect::new(x, area.y + 1, width, 1);
        let selected = index == active;
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

    if area.width > 100 {
        let hint = "Tab 切页   / 搜索   ? 帮助";
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
    fill(
        frame,
        Rect::new(area.x, area.y + 2, area.width, 1),
        theme.color("surface0"),
    );
}

fn draw_footer(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, _hits: &[HitBox]) {
    fill(frame, area, theme.color("crust"));
    let status_rect = Rect::new(area.x + 2, area.y, area.width.saturating_sub(26), 1);
    let (text, color) = if let Some(busy) = app.busy_text() {
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

    if area.width > 68 {
        let info = format!(
            "{} 缓存 {} 个文件 · {}   {} {}",
            icons::DATABASE,
            app.cache_stats.files,
            format::format_size(app.cache_stats.bytes),
            icons::GLOBE,
            proxy_label(app)
        );
        let width = area.width.saturating_sub(2);
        let rect = Rect::new(area.x + 2, area.y + 1, width, 1);
        let hint = hints_for(app.page);
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
        let hint = hints_for(app.page);
        frame.render_widget(
            Paragraph::new(format::truncate(hint, area.width as usize - 4))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
        );
    }
}

fn hints_for(page: Page) -> &'static str {
    match page {
        Page::Repositories => "j/k 选择   Enter 打开   o 主页   y 复制   r 刷新   / 搜索   ? 帮助",
        Page::Starred => "j/k 选择   Enter 打开   o 主页   y 复制   r 刷新   ? 帮助",
        Page::Search => "输入后回车   Esc 离开输入   j/k 选择   Enter 打开   ? 帮助",
        Page::Settings => "j/k 选择   h/l 调整   Enter 操作   Ctrl+S 保存   Esc 离开",
        Page::Repo => "Esc 返回   j/k 滚动   o 主页   y 复制   r 刷新   ? 帮助",
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
        .unwrap_or_else(|| "未登录".to_string());
    frame.render_widget(
        Paragraph::new(icons::label(icons::USER, "当前账号"))
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
        "GitHub CLI 已连接"
    } else {
        "先运行 gh auth login"
    };
    frame.render_widget(
        Paragraph::new(format::truncate(auth, width as usize))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 2, y, width, 1),
    );
    y += 2;

    frame.render_widget(
        Paragraph::new(icons::label(icons::CODE, "语言筛选"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(area.x + 2, y, width, 1),
    );
    y += 1;

    let total = app.repos.len();
    let all_active = app.language_filter.is_none();
    let all_rect = Rect::new(area.x, y, area.width.saturating_sub(1), 1);
    draw_sidebar_item(
        frame,
        theme,
        all_rect,
        "全部",
        total,
        all_active,
        app.hover.as_ref() == Some(&Action::Language(None)),
    );
    register(hits, all_rect, Action::Language(None));
    y += 1;

    let counts = app.language_counts();
    for (language, count) in counts {
        if y >= area.y + area.height.saturating_sub(5) {
            break;
        }
        let active = app
            .language_filter
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case(&language))
            .unwrap_or(false);
        let action = Action::Language(Some(language.clone()));
        let hovered = app.hover.as_ref() == Some(&action);
        let rect = Rect::new(area.x, y, area.width.saturating_sub(1), 1);
        draw_sidebar_item(frame, theme, rect, &language, count, active, hovered);
        register(hits, rect, action);
        y += 1;
    }

    let cache_top = area.y + area.height.saturating_sub(5);
    if cache_top > y {
        frame.render_widget(
            Paragraph::new(icons::label(icons::DATABASE, "本地缓存"))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 2, cache_top, width, 1),
        );
        let stats = format!(
            "{} 个文件 · {}",
            app.cache_stats.files,
            format::format_size(app.cache_stats.bytes)
        );
        frame.render_widget(
            Paragraph::new(format::truncate(&stats, width as usize))
                .style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(area.x + 2, cache_top + 1, width, 1),
        );
        frame.render_widget(
            Paragraph::new("在设置页管理缓存").style(Style::default().fg(theme.color("overlay0"))),
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
        &icons::label(icons::REPO, "我的仓库"),
        "人家把账号下的仓库都整理好啦。",
        "刷新",
        Action::Refresh,
        hits,
    );

    let (public, private, languages, stars) = app.repo_stats();
    let stats = format!(
        "公开 {}   私有 {}   语言 {}   星标合计 {}",
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
        &icons::label(icons::CLOCK, "排序"),
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

    let list = Rect::new(
        inner.x,
        inner.y + 5,
        inner.width,
        inner.height.saturating_sub(5),
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
        Paragraph::new(icons::label(icons::STAR, "收藏概览"))
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
        Paragraph::new("个仓库收了星星").style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!("{} 语言  {}", icons::CODE, languages))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 5, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{} 星标  {}",
            icons::STAR,
            format::human_count(stars)
        ))
        .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 6, inner.width, 1),
    );

    let mut y = inner.y + 9;
    frame.render_widget(
        Paragraph::new(icons::label(icons::INFO, "人家的小提示"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 1;
    let note = [
        "收藏列表来自 GitHub。",
        "点仓库卡片可以看详情，",
        "双击或者按 Enter 进入",
        "仓库页。",
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
        &icons::label(icons::STAR, "收藏仓库"),
        "点过星星的仓库，都收在这里啦。",
        "刷新",
        Action::Refresh,
        hits,
    );
    let (count, _, _) = app.starred_stats();
    frame.render_widget(
        Paragraph::new(format!("当前显示 {} 个收藏", count))
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );
    draw_sort_chips(
        frame,
        app,
        theme,
        hits,
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
        &icons::label(icons::CLOCK, "排序"),
        &[
            (
                RepoSort::Updated.label(),
                Action::SetStarSort(RepoSort::Updated),
                app.star_sort == RepoSort::Updated,
            ),
            (
                RepoSort::Stars.label(),
                Action::SetStarSort(RepoSort::Stars),
                app.star_sort == RepoSort::Stars,
            ),
            (
                RepoSort::Name.label(),
                Action::SetStarSort(RepoSort::Name),
                app.star_sort == RepoSort::Name,
            ),
        ],
    );
    let list = Rect::new(
        inner.x,
        inner.y + 5,
        inner.width,
        inner.height.saturating_sub(5),
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
        &icons::label(icons::SEARCH, "搜索仓库"),
        "想找什么仓库呢？告诉人家就好啦。",
        "刷新",
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
        &icons::label(icons::CLOCK, "排序"),
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

    let content = Rect::new(
        inner.x,
        sort_y + 2,
        inner.width,
        inner.height.saturating_sub(sort_y - inner.y + 2),
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
    let placeholder = "输入关键词，回车开始找仓库";
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
        &icons::label(icons::SEARCH, "搜索"),
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
    let max_offset = indices.len().saturating_sub(viewport);
    app.list_offset = app.list_offset.min(max_offset);
    let selected = app.selected_index();

    if indices.is_empty() {
        draw_empty_state(frame, app, theme, area, hits, empty);
        return;
    }

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
        draw_repo_card(frame, theme, card, &repo, is_selected, is_hovered);
        register(hits, card, action);
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
}

fn draw_empty_state(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    hits: &mut Vec<HitBox>,
    kind: EmptyKind,
) {
    if matches!(kind, EmptyKind::Search) {
        draw_search_history(frame, app, theme, area, hits);
        return;
    }
    let loading = match kind {
        EmptyKind::Repositories => app.repos_loading,
        EmptyKind::Starred => app.starred_loading,
        EmptyKind::Search => app.search_loading,
    };
    let (title, detail) = if loading {
        ("人家正在搬数据...", "第一次同步会慢一点点，等一等哦。")
    } else if !app.client.token_present() {
        (
            "还没有登录 GitHub",
            "在终端运行 gh auth login，再重新打开人家。",
        )
    } else {
        match kind {
            EmptyKind::Repositories => (
                "这里还没有仓库呢",
                "先去 GitHub 建一个，或者按 r 刷新试试。",
            ),
            EmptyKind::Starred => (
                "还没有收藏任何仓库",
                "在 GitHub 上点一颗星星，这里就会热闹起来。",
            ),
            EmptyKind::Search => ("没有搜到仓库", "换个关键词再试试嘛。"),
        }
    };
    draw_center_message(frame, theme, area, title, detail);
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
            "人家正在找仓库..."
        } else {
            "还没有开始搜索呢"
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
            Paragraph::new("在上面的搜索框里输入关键词，按回车就好啦。")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x + 1, y, area.width.saturating_sub(2), 1),
        );
        y += 3;
        if !app.config.search_history.is_empty() {
            frame.render_widget(
                Paragraph::new("最近搜过").style(
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
            "人家正在找仓库...",
            "搜索结果一会儿就回来啦。",
        );
        return;
    }
    if app.search_results.is_empty() {
        draw_center_message(
            frame,
            theme,
            area,
            "没有搜到仓库",
            "换个关键词，或者去掉一些限定条件试试嘛。",
        );
    }
}

fn draw_repo_card(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    repo: &Repo,
    selected: bool,
    hovered: bool,
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
    let mut title = vec![
        Span::styled(
            format!("{} ", icons::REPO),
            Style::default().fg(theme.color("blue")),
        ),
        Span::styled(
            repo.owner_name().to_string(),
            Style::default().fg(theme.color("overlay1")),
        ),
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
    let (visibility_icon, visibility_text) = if repo.private {
        (icons::LOCK, "私有")
    } else {
        (icons::GLOBE, "公开")
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
            "挑一个仓库看看",
            "点左边卡片，人家就把详情摆在这里。",
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
        Paragraph::new(icons::label(icons::REPO, "仓库详情"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
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
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
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
                format!("{} 星标", icons::STAR),
                format::human_count(repo.stargazers_count),
                format!("{} 复刻", icons::FORK),
                format::human_count(repo.forks_count),
            ),
            (
                format!("{} 议题", icons::ISSUE),
                format::human_count(repo.open_issues_count),
                format!("{} 关注", icons::EYE),
                format::human_count(repo.watchers_count),
            ),
            (
                format!("{} 更新", icons::CLOCK),
                format::relative_time(repo.updated_at()),
                format!("{} 分支", icons::BRANCH),
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
        let buttons = [
            (
                &*icons::label(icons::EXTERNAL, "打开"),
                Action::OpenCurrent,
                false,
            ),
            (
                &*icons::label(icons::COPY, "复制"),
                Action::CopyClone,
                false,
            ),
            (&*icons::label(icons::TAG, "详情"), Action::OpenPage, true),
        ];
        for (label, action, primary) in buttons {
            let width = format::text_width(label) as u16 + 4;
            if x + width > inner.x + inner.width {
                break;
            }
            draw_button(
                frame,
                app,
                theme,
                hits,
                Rect::new(x, buttons_y, width, 1),
                label,
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
        Paragraph::new(icons::label(icons::TAG, "发行版")).style(
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
            Paragraph::new(format!("{} 人家正在取...", app.spinner()))
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(inner.x, releases_y + 2, inner.width, 1),
        );
        return;
    }
    if app.releases.is_empty() {
        frame.render_widget(
            Paragraph::new("这个仓库还没有发行版。")
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
            format!("资产 {} 个", release.assets.len()),
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
            Paragraph::new("发行版资产经 v4 加速")
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
            "仓库不见了",
            "按 Esc 回到列表再选一次吧。",
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
            "这里有点挤",
            "把终端拉大一点，人家才摆得下仓库页。",
        );
        return;
    }

    let back_action = Action::Back;
    let back_rect = Rect::new(inner.x, inner.y, 10, 1);
    let back_hover = app.hover.as_ref() == Some(&back_action);
    frame.render_widget(
        Paragraph::new(icons::label(icons::BACK, "返回")).style(if back_hover {
            Style::default().fg(theme.color("blue"))
        } else {
            Style::default().fg(theme.color("subtext0"))
        }),
        back_rect,
    );
    register(hits, back_rect, back_action);
    let crumb = format!(
        "{} / {} / {}",
        app.return_page.title(),
        repo.owner_name(),
        repo.name
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &crumb,
            inner.width.saturating_sub(12) as usize,
        ))
        .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(inner.x + 12, inner.y, inner.width.saturating_sub(14), 1),
    );

    let title_y = inner.y + 1;
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
        Rect::new(inner.x, title_y, inner.width, 1),
    );
    let visibility_icon = if repo.private {
        icons::LOCK
    } else {
        icons::GLOBE
    };
    let badge = format!(
        "{visibility_icon} {}   {} {}{}",
        repo.visibility(),
        icons::CODE,
        repo.language_label(),
        if repo.archived {
            format!("   {} 已归档", icons::ARCHIVE)
        } else {
            String::new()
        }
    );
    frame.render_widget(
        Paragraph::new(badge).style(Style::default().fg(theme.color("sapphire"))),
        Rect::new(inner.x, title_y + 1, inner.width, 1),
    );
    let desc_width = inner.width.min(100);
    let desc = format::wrap_text(
        &format::clean_text(repo.description_text()),
        desc_width as usize,
        2,
    );
    let mut y = title_y + 2;
    for line in desc {
        frame.render_widget(
            Paragraph::new(line).style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(inner.x, y, desc_width, 1),
        );
        y += 1;
    }
    if y < title_y + 4 {
        y = title_y + 4;
    }

    let stats = [
        (
            format!("{} 星标", icons::STAR),
            format::human_count(repo.stargazers_count),
        ),
        (
            format!("{} 复刻", icons::FORK),
            format::human_count(repo.forks_count),
        ),
        (
            format!("{} 关注", icons::EYE),
            format::human_count(repo.watchers_count),
        ),
        (
            format!("{} 议题", icons::ISSUE),
            format::human_count(repo.open_issues_count),
        ),
    ];
    let stat_width = ((inner.width.saturating_sub(3)) / 4).max(10);
    for (index, (label, value)) in stats.iter().enumerate() {
        let rect = Rect::new(inner.x + index as u16 * (stat_width + 1), y, stat_width, 2);
        if rect.x + rect.width > inner.x + inner.width {
            break;
        }
        fill(frame, rect, theme.color("mantle"));
        frame.render_widget(
            Paragraph::new(format!("{label} {value}")).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(rect.x + 1, rect.y, rect.width.saturating_sub(2), 1),
        );
    }
    y += 3;

    let mut button_x = inner.x;
    let buttons = [
        (
            &*icons::label(icons::EXTERNAL, "打开主页"),
            Action::OpenCurrent,
            false,
        ),
        (
            &*icons::label(icons::COPY, "复制克隆"),
            Action::CopyClone,
            false,
        ),
        (
            &*icons::label(icons::DOWNLOAD, "下载源码"),
            Action::SourceZip,
            false,
        ),
        (
            &*icons::label(icons::REFRESH, "刷新"),
            Action::Refresh,
            true,
        ),
    ];
    for (label, action, primary) in buttons {
        let width = format::text_width(label) as u16 + 4;
        if button_x + width > inner.x + inner.width {
            break;
        }
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(button_x, y, width, 1),
            label,
            action,
            primary,
        );
        button_x += width + 1;
    }
    y += 2;
    fill(
        frame,
        Rect::new(inner.x, y, inner.width, 1),
        theme.color("surface0"),
    );
    y += 1;

    let content = Rect::new(
        inner.x,
        y,
        inner.width,
        (inner.y + inner.height).saturating_sub(y),
    );
    if content.height < 4 {
        return;
    }
    let side_width = if inner.width >= 100 { 38 } else { 0 };
    let columns =
        Layout::horizontal([Constraint::Min(30), Constraint::Length(side_width)]).split(content);
    draw_releases_panel(frame, app, theme, &repo, columns[0], hits);
    app.layout.page_releases = columns[0];
    if side_width > 0 {
        draw_repo_about(frame, app, theme, &repo, columns[1], hits);
        app.layout.detail = columns[1];
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
    frame.render_widget(
        Paragraph::new(icons::label(icons::TAG, "发行版")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x, area.y, area.width, 1),
    );
    let loading = app
        .releases_loading
        .as_ref()
        .is_some_and(|name| name == &repo.display_full_name());
    let content = Rect::new(
        area.x,
        area.y + 2,
        area.width,
        area.height.saturating_sub(2),
    );
    if loading {
        frame.render_widget(
            Paragraph::new(format!("{} 人家正在取发行版...", app.spinner()))
                .style(Style::default().fg(theme.color("peach"))),
            content,
        );
        return;
    }
    if app.releases.is_empty() {
        frame.render_widget(
            Paragraph::new("这个仓库还没有发行版，等作者发一个吧。")
                .style(Style::default().fg(theme.color("overlay0"))),
            content,
        );
        return;
    }
    let mut lines: Vec<Line> = Vec::new();
    let mut actions: Vec<(usize, Action)> = Vec::new();
    for (index, release) in app.releases.iter().enumerate() {
        let mut title = vec![Span::styled(
            release.tag_name.clone(),
            Style::default()
                .fg(theme.color("blue"))
                .add_modifier(Modifier::BOLD),
        )];
        if index == 0 {
            title.push(Span::styled(
                "  最新",
                Style::default().fg(theme.color("green")),
            ));
        }
        if release.prerelease {
            title.push(Span::styled(
                "  预发布",
                Style::default().fg(theme.color("peach")),
            ));
        }
        title.push(Span::styled(
            format!("   {}", format::relative_time(release.date())),
            Style::default().fg(theme.color("overlay0")),
        ));
        actions.push((lines.len(), Action::OpenReleasePage(index)));
        lines.push(Line::from(title));
        lines.push(Line::from(Span::styled(
            format::truncate(&format::clean_text(release.title()), area.width as usize),
            Style::default().fg(theme.color("subtext0")),
        )));
        let excerpt = release
            .body
            .as_deref()
            .map(plain_excerpt)
            .map(|text| format::clean_text(&text))
            .unwrap_or_default();
        if !excerpt.is_empty() {
            for line in format::wrap_text(&excerpt, area.width.saturating_sub(2) as usize, 2) {
                lines.push(Line::from(Span::styled(
                    format!("  {line}"),
                    Style::default().fg(theme.color("overlay0")),
                )));
            }
        }
        if release.assets.is_empty() {
            lines.push(Line::from(Span::styled(
                "  没有二进制资产",
                Style::default().fg(theme.color("overlay0")),
            )));
        } else {
            for (asset_index, asset) in release.assets.iter().take(5).enumerate() {
                let action = Action::OpenAsset {
                    release: index,
                    asset: asset_index,
                };
                let line = Line::from(vec![
                    Span::styled(
                        format!("  {} 下载 ", icons::DOWNLOAD),
                        Style::default().fg(theme.color("blue")),
                    ),
                    Span::styled(
                        format::truncate(
                            &format::clean_text(&asset.name),
                            area.width.saturating_sub(28) as usize,
                        ),
                        Style::default().fg(theme.color("subtext1")),
                    ),
                    Span::styled(
                        format!(
                            "  {}   {} 次  经 v4",
                            format::format_size(asset.size),
                            format::human_count(asset.download_count)
                        ),
                        Style::default().fg(theme.color("overlay0")),
                    ),
                ]);
                actions.push((lines.len(), action));
                lines.push(line);
            }
            if release.assets.len() > 5 {
                lines.push(Line::from(Span::styled(
                    format!("  还有 {} 个资产", release.assets.len() - 5),
                    Style::default().fg(theme.color("overlay0")),
                )));
            }
        }
        lines.push(Line::default());
    }
    app.page_scroll_max = lines.len().saturating_sub(content.height as usize) as u16;
    app.page_scroll = app.page_scroll.min(app.page_scroll_max);
    render_scrolled(frame, content, lines, actions, app.page_scroll, theme, hits);
}

fn draw_repo_about(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    repo: &Repo,
    area: Rect,
    hits: &mut Vec<HitBox>,
) {
    frame.render_widget(
        Paragraph::new(icons::label(icons::INFO, "关于")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(area.x, area.y, area.width, 1),
    );
    let mut y = area.y + 2;
    let desc = format::clean_text(repo.description_text());
    for line in format::wrap_text(&desc, area.width as usize, 3) {
        frame.render_widget(
            Paragraph::new(line).style(Style::default().fg(theme.color("subtext0"))),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
    }
    y += 1;
    let facts = [
        (
            format!("{} 默认分支", icons::BRANCH),
            repo.default_branch.clone(),
        ),
        (format!("{} 许可", icons::LICENSE), repo.license_label()),
        (
            format!("{} 更新", icons::CLOCK),
            format::relative_time(repo.updated_at()),
        ),
        (
            format!("{} 大小", icons::DATABASE),
            format::format_size(repo.size.saturating_mul(1024)),
        ),
        (
            format!("{} 可见性", icons::GLOBE),
            repo.visibility().to_string(),
        ),
    ];
    for (label, value) in facts {
        if y >= area.y + area.height {
            return;
        }
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{label}  "),
                    Style::default().fg(theme.color("overlay0")),
                ),
                Span::styled(
                    format::truncate(&value, area.width.saturating_sub(9) as usize),
                    Style::default().fg(theme.color("subtext1")),
                ),
            ])),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
    }
    if !repo.topics.is_empty() && y < area.y + area.height {
        y += 1;
        frame.render_widget(
            Paragraph::new(icons::label(icons::TAG, "主题"))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
        let mut x = area.x;
        for topic in repo.topics.iter().take(6) {
            let label = format!("#{}", format::clean_text(topic));
            let width = format::text_width(&label) as u16 + 2;
            if x + width > area.x + area.width {
                break;
            }
            let rect = Rect::new(x, y, width, 1);
            let action = Action::Topic(topic.clone());
            let hovered = app.hover.as_ref() == Some(&action);
            fill(
                frame,
                rect,
                if hovered {
                    theme.color("surface1")
                } else {
                    theme.color("surface0")
                },
            );
            frame.render_widget(
                Paragraph::new(format!(" {label}"))
                    .style(Style::default().fg(theme.color("mauve"))),
                rect,
            );
            register(hits, rect, action);
            x += width + 1;
        }
        y += 2;
    }

    if y < area.y + area.height {
        frame.render_widget(
            Paragraph::new(icons::label(icons::TERMINAL, "克隆"))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
    }
    let clone = if repo.ssh_url.is_empty() {
        format!("https://github.com/{}.git", repo.display_full_name())
    } else {
        repo.ssh_url.clone()
    };
    if y < area.y + area.height {
        frame.render_widget(
            Paragraph::new(format::truncate(&clone, area.width as usize))
                .style(Style::default().fg(theme.color("subtext1"))),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
    }
    let mut x = area.x;
    for (label, action) in [
        (&*icons::label(icons::COPY, "复制克隆"), Action::CopyClone),
        (
            &*icons::label(icons::DOWNLOAD, "下载源码"),
            Action::SourceZip,
        ),
    ] {
        if y >= area.y + area.height {
            return;
        }
        let width = format::text_width(label) as u16 + 4;
        if x + width > area.x + area.width {
            break;
        }
        draw_button(
            frame,
            app,
            theme,
            hits,
            Rect::new(x, y, width, 1),
            label,
            action,
            false,
        );
        x += width + 1;
    }
    if y + 2 < area.y + area.height {
        frame.render_widget(
            Paragraph::new(format!("{} 发行版下载会走 v4.gh-proxy.org", icons::NETWORK))
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(area.x, y + 2, area.width, 1),
        );
    }
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
        Paragraph::new(icons::label(icons::SETTINGS, "设置")).style(
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new("改动会写进本机的配置文件里，保存之后才算数哦。")
            .style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let actions_y = inner.y + inner.height.saturating_sub(2);
    let mut y = inner.y + 3;

    let config_rect = Rect::new(inner.x, y, inner.width, 3);
    fill(frame, config_rect, theme.color("mantle"));
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.color("surface0"))),
        config_rect,
    );
    frame.render_widget(
        Paragraph::new(icons::label(icons::TERMINAL, "配置文件"))
            .style(Style::default().fg(theme.color("overlay0"))),
        Rect::new(
            config_rect.x + 2,
            config_rect.y,
            config_rect.width.saturating_sub(4),
            1,
        ),
    );
    frame.render_widget(
        Paragraph::new(format::truncate(
            &app.config_file_label(),
            config_rect.width.saturating_sub(4) as usize,
        ))
        .style(Style::default().fg(theme.color("subtext1"))),
        Rect::new(
            config_rect.x + 2,
            config_rect.y + 1,
            config_rect.width.saturating_sub(4),
            1,
        ),
    );
    y = config_rect.y + 4;

    if y + 5 < actions_y {
        frame.render_widget(
            Paragraph::new(icons::label(icons::PALETTE, "主题")).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(inner.x, y, inner.width, 1),
        );
        frame.render_widget(
            Paragraph::new("Catppuccin 四套配色，点一下就能换。")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x + 8, y, inner.width.saturating_sub(8), 1),
        );
        y += 1;
        let gap = 1_u16;
        let card_width = (inner.width.saturating_sub(gap * 3)) / 4;
        for (index, flavor) in FLAVORS.iter().enumerate() {
            let rect = Rect::new(
                inner.x + index as u16 * (card_width + gap),
                y,
                card_width,
                4,
            );
            if rect.x + rect.width > inner.x + inner.width {
                break;
            }
            draw_theme_card(frame, app, theme, rect, flavor, hits);
        }
        y += 5;
    }

    let mut proxy_input_rect = None;
    if y + 3 < actions_y {
        frame.render_widget(
            Paragraph::new(icons::label(icons::NETWORK, "网络")).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(inner.x, y, inner.width, 1),
        );
        frame.render_widget(
            Paragraph::new("公开 API 与发行版走代理；认证请求直连 GitHub。")
                .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x + 8, y, inner.width.saturating_sub(8), 1),
        );
        y += 1;
        let proxy_rect = Rect::new(inner.x, y, inner.width, 2);
        draw_proxy_row(frame, app, theme, proxy_rect, hits);
        proxy_input_rect = Some(Rect::new(
            inner.x + 2,
            proxy_rect.y + 1,
            inner.width.saturating_sub(4),
            1,
        ));
        y = proxy_rect.y + 2;
    }

    if y + 4 < actions_y {
        frame.render_widget(
            Paragraph::new(icons::label(icons::DATABASE, "缓存")).style(
                Style::default()
                    .fg(theme.color("text"))
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(inner.x, y, inner.width, 1),
        );
        frame.render_widget(
            Paragraph::new(format!(
                "{} 个文件 · {}",
                app.cache_stats.files,
                format::format_size(app.cache_stats.bytes)
            ))
            .style(Style::default().fg(theme.color("overlay0"))),
            Rect::new(inner.x + 8, y, inner.width.saturating_sub(8), 1),
        );
        y += 1;
        for (index, (label, kind)) in [
            ("仓库缓存", TtlKind::Repositories),
            ("详情缓存", TtlKind::Detail),
            ("搜索缓存", TtlKind::Search),
        ]
        .into_iter()
        .enumerate()
        {
            if y >= actions_y {
                break;
            }
            draw_ttl_row(
                frame,
                app,
                theme,
                hits,
                Rect::new(inner.x, y, inner.width, 1),
                label,
                kind,
                2 + index,
            );
            y += 1;
        }
    }

    let mut x = inner.x;
    let save_action = Action::SaveConfig;
    let save_hover = app.hover.as_ref() == Some(&save_action);
    let save_width = 16;
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
        Paragraph::new(format!("{} 保存配置", icons::SAVE))
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
    let clear_width = 14;
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
        Paragraph::new(format!("{} 清理缓存", icons::TRASH))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.color("red"))),
        Rect::new(x, actions_y, clear_width, 1),
    );
    register(hits, Rect::new(x, actions_y, clear_width, 1), clear_action);
    if app.config_dirty {
        frame.render_widget(
            Paragraph::new("有未保存的改动")
                .alignment(Alignment::Right)
                .style(Style::default().fg(theme.color("peach"))),
            Rect::new(inner.x, actions_y, inner.width, 1),
        );
    }
    if app.editing_proxy
        && let Some(input_rect) = proxy_input_rect
    {
        let (_, cursor_column) = visible_input(&app.proxy_input, true, input_rect.width);
        let x = input_rect
            .x
            .saturating_add(cursor_column.min(input_rect.width.saturating_sub(1)));
        frame.set_cursor_position(Position::new(x, input_rect.y));
    }
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
    let label = Theme::label(flavor);
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

fn draw_proxy_row(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, hits: &mut Vec<HitBox>) {
    let row = Rect::new(area.x, area.y, area.width, 1);
    if app.settings_row == 1 {
        fill(frame, row, theme.color("surface0"));
    }
    frame.render_widget(
        Paragraph::new("API 代理").style(Style::default().fg(theme.color("subtext1"))),
        Rect::new(row.x + 1, row.y, 10, 1),
    );
    if app.editing_proxy {
        fill(
            frame,
            Rect::new(area.x, area.y + 1, area.width, 1),
            theme.color("mantle"),
        );
        let input_width = area.width.saturating_sub(3);
        let (visible, _) = visible_input(&app.proxy_input, true, input_width);
        frame.render_widget(
            Paragraph::new(format::truncate(&visible, input_width as usize))
                .style(Style::default().fg(theme.color("text"))),
            Rect::new(area.x + 2, area.y + 1, input_width, 1),
        );
        return;
    }
    let proxy = format::truncate(app.client.proxy(), area.width.saturating_sub(14) as usize);
    frame.render_widget(
        Paragraph::new(proxy).style(Style::default().fg(theme.color("subtext0"))),
        Rect::new(area.x + 10, row.y, area.width.saturating_sub(20), 1),
    );
    let edit_action = Action::StartProxyEdit;
    let edit_width = 8;
    let edit_rect = Rect::new(
        area.x + area.width.saturating_sub(edit_width),
        row.y,
        edit_width,
        1,
    );
    let hovered = app.hover.as_ref() == Some(&edit_action);
    fill(
        frame,
        edit_rect,
        if hovered {
            theme.color("surface1")
        } else {
            theme.color("surface0")
        },
    );
    frame.render_widget(
        Paragraph::new("编辑")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.color("text"))),
        edit_rect,
    );
    register(hits, row, Action::SetSettingsRow(1));
    register(hits, edit_rect, edit_action);
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
        Rect::new(area.x + 1, area.y, 12, 1),
    );
    let value = match kind {
        TtlKind::Repositories => app.config.repos_ttl_secs,
        TtlKind::Detail => app.config.detail_ttl_secs,
        TtlKind::Search => app.config.search_ttl_secs,
    };
    frame.render_widget(
        Paragraph::new(format::ttl_label(value)).style(Style::default().fg(theme.color("text"))),
        Rect::new(area.x + 14, area.y, 12, 1),
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

fn plain_excerpt(body: &str) -> String {
    let mut lines = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("<!--")
            || trimmed.starts_with("[//]:")
            || trimmed.starts_with("**Full Changelog**")
        {
            continue;
        }
        let cleaned = trimmed
            .trim_start_matches('#')
            .trim_start_matches('-')
            .trim_start_matches('*')
            .trim();
        if !cleaned.is_empty() {
            lines.push(cleaned.replace('`', ""));
        }
        if lines.len() >= 4 {
            break;
        }
    }
    lines.join(" ")
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

fn draw_help(frame: &mut Frame, _app: &App, theme: &Theme, area: Rect, hits: &mut Vec<HitBox>) {
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
        ("页面", "1 仓库   2 收藏   3 搜索   4 设置   Tab 切页"),
        ("移动", "j / k   上 / 下   g / G   首 / 尾"),
        ("仓库", "Enter 打开仓库页   o 浏览器   y 复制克隆地址"),
        ("搜索", "/ 聚焦搜索   Enter 搜索   Esc 离开输入框"),
        ("设置", "j / k 选择   h / l 调整   Enter 操作   Ctrl+S 保存"),
        ("鼠标", "单击选择，双击打开仓库页，滚轮滚动列表"),
        ("返回", "Esc 返回列表   ? 关闭帮助   q 退出"),
    ];
    let mut y = inner.y;
    frame.render_widget(
        Paragraph::new(icons::label(icons::INFO, "操作说明")).style(
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
        _app,
        theme,
        hits,
        close,
        "关闭",
        Action::CloseOverlay,
        false,
    );
    let hit = Rect::new(rect.x, rect.y, rect.width, rect.height);
    register(hits, hit, Action::CloseOverlay);
}

fn draw_too_small(frame: &mut Frame, area: Rect, theme: &Theme) {
    fill(frame, area, theme.color("base"));
    let lines = vec![
        Line::from(Span::styled(
            "终端再大一点嘛",
            Style::default()
                .fg(theme.color("text"))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("至少要 {MIN_WIDTH} x {MIN_HEIGHT}，现在有点挤。"),
            Style::default().fg(theme.color("overlay0")),
        )),
        Line::from(Span::styled(
            "按 Ctrl+C 可以退出。",
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
    if language == "未标注" {
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
    let max_width = max_width.max(1) as usize;
    if !focused {
        return (format::truncate(&input.value, max_width), 0);
    }
    let prefix_width = format::text_width(&input.value[..input.cursor]);
    let start_column = prefix_width.saturating_sub(max_width.saturating_sub(1));
    let mut visible = String::new();
    let mut total = 0;
    let mut cursor_column = 0;
    for (index, ch) in input.value.char_indices() {
        let width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if total + width <= start_column {
            total += width;
            continue;
        }
        if index < input.cursor {
            if cursor_column + width as u16 > max_width as u16 {
                break;
            }
            cursor_column += width as u16;
        }
        if format::text_width(&visible) + width > max_width {
            break;
        }
        visible.push(ch);
    }
    (visible, cursor_column.min(max_width as u16))
}
